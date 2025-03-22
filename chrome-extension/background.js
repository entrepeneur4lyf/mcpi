// Background script for MCPI Client extension

// Track MCPI discovery state for each tab
let mcpiTabStates = {};

// WebSocket connection state
let websocketInfo = {
  connection: null,
  jsonRpcId: 1,
  lastActivity: null,
  initialized: false,
  reconnectAttempts: 0,
  discoveredTools: [], // Store discovered tools from server
  discoveredResources: [] // Store discovered resources
};

// Log initialization to confirm the script is running
console.log("MCPI Client background script initialized");

// Listen for tab updates to detect domain changes
chrome.tabs.onUpdated.addListener((tabId, changeInfo, tab) => {
  // Only run when the tab has completed loading
  if (changeInfo.status === 'complete' && tab.url) {
    console.log(`Tab updated: ${tabId}`, tab.url);
    checkForMcpiSupport(tab.url, tabId);
  }
});

// Listen for tab activation to update icon if needed
chrome.tabs.onActivated.addListener((activeInfo) => {
  console.log(`Tab activated: ${activeInfo.tabId}`);
  updateIconForTab(activeInfo.tabId);
});

// Improved message handling with proper error checking
chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
  console.log("Received message:", message);
  
  try {
    if (message.action === 'getMcpiState') {
      // Get the current tab's MCPI state
      chrome.tabs.query({ active: true, currentWindow: true }, (tabs) => {
        if (tabs && tabs.length > 0) {
          const tabId = tabs[0].id;
          const state = mcpiTabStates[tabId] || { supported: false };
          
          // Add connection status to the state
          state.connectionStatus = getConnectionStatus();
          
          // Add discovered tools and resources
          state.discoveredTools = websocketInfo.discoveredTools;
          state.discoveredResources = websocketInfo.discoveredResources;
          
          console.log("Sending state:", state);
          sendResponse(state);
        } else {
          console.log("No active tab found");
          sendResponse({ supported: false });
        }
      });
      
      return true; // Return true to indicate async response
    }
    
    if (message.action === 'connectToMcpi') {
      // Connect to MCPI server
      const tabId = message.tabId;
      const state = mcpiTabStates[tabId];
      
      if (state && state.supported && state.websocketUrl) {
        console.log(`Connecting to ${state.websocketUrl}`);
        const result = connectToMcpiServer(state.websocketUrl);
        sendResponse({ success: result });
      } else {
        console.log("No MCPI support found");
        sendResponse({ success: false, error: 'No MCPI support found' });
      }
      
      return true;
    }
    
    if (message.action === 'getConnectionStatus') {
      sendResponse(getConnectionStatus());
      return true;
    }
    
    if (message.action === 'getDiscoveredTools') {
      sendResponse({ tools: websocketInfo.discoveredTools });
      return true;
    }
    
    if (message.action === 'getDiscoveredResources') {
      sendResponse({ resources: websocketInfo.discoveredResources });
      return true;
    }
    
    if (message.action === 'sendRequest') {
      if (websocketInfo.connection && websocketInfo.connection.readyState === WebSocket.OPEN) {
        try {
          const requestId = sendJsonRpcRequest(message.method, message.params);
          sendResponse({ success: true, requestId });
        } catch (error) {
          console.error('Error sending request:', error);
          sendResponse({ success: false, error: error.message });
        }
      } else {
        console.log("No active connection");
        sendResponse({ success: false, error: 'No active connection' });
      }
      return true;
    }
  } catch (error) {
    console.error("Error handling message:", error);
    sendResponse({ error: error.message });
    return true;
  }
});

// Check if a URL's domain supports MCPI
async function checkForMcpiSupport(url, tabId) {
  try {
    // Extract domain from URL
    const domain = new URL(url).hostname;
    
    console.log(`Checking MCPI support for domain: ${domain}`);
    
    // Check for MCPI TXT record
    const mcpiRecord = await queryMcpiTxtRecord(domain);
    
    if (mcpiRecord) {
      // We found MCPI support
      console.log(`MCPI support found for ${domain}:`, mcpiRecord);
      
      // Extract discovery URL and derive WebSocket URL
      const discoveryUrl = mcpiRecord.discoveryUrl;
      const websocketUrl = deriveWebsocketUrl(discoveryUrl);
      
      // Fetch server details via discovery endpoint
      const serverDetails = await fetchServerDetails(discoveryUrl);
      
      // Store the state for this tab
      mcpiTabStates[tabId] = {
        supported: true,
        domain: domain,
        discoveryUrl: discoveryUrl,
        websocketUrl: websocketUrl,
        version: mcpiRecord.version,
        serverDetails: serverDetails
      };
      
      // Update extension icon to show MCPI is available
      setIconActive(tabId);
    } else {
      // No MCPI support found
      console.log(`No MCPI support found for ${domain}`);
      
      // Update state and icon
      mcpiTabStates[tabId] = { supported: false };
      setIconInactive(tabId);
    }
  } catch (error) {
    console.error('Error checking MCPI support:', error);
    
    // Update state and icon for error case
    mcpiTabStates[tabId] = { supported: false, error: error.message };
    setIconInactive(tabId);
  }
}

// Update icon based on tab's stored state
function updateIconForTab(tabId) {
  const state = mcpiTabStates[tabId];
  
  if (state && state.supported) {
    setIconActive(tabId);
  } else {
    setIconInactive(tabId);
  }
}

// Set icon to active state (green)
function setIconActive(tabId) {
  try {
    // Use "MCP" as badge text
    chrome.action.setBadgeText({ tabId: tabId, text: "MCP" });
    chrome.action.setBadgeBackgroundColor({ tabId: tabId, color: "#2ecc71" }); // Green background
    
    // You can use title to provide more context
    chrome.action.setTitle({ tabId: tabId, title: "MCP Protocol Available" });
  } catch (error) {
    console.error("Error setting active state:", error);
  }
}

// Set icon to inactive state (red)
function setIconInactive(tabId) {
  try {
    // Keep the same text but change color to indicate unavailability
    chrome.action.setBadgeText({ tabId: tabId, text: "MCP" });
    chrome.action.setBadgeBackgroundColor({ tabId: tabId, color: "#e74c3c" }); // Red background
    
    // Update title
    chrome.action.setTitle({ tabId: tabId, title: "MCP Protocol Not Available" });
  } catch (error) {
    console.error("Error setting inactive state:", error);
  }
}

// Query for MCP TXT record using DNS-over-HTTPS
async function queryMcpiTxtRecord(domain) {
  try {
    console.log('Querying MCP TXT record for:', domain);
    
    // Using Google's DNS-over-HTTPS service to query TXT records
    const dnsQueryUrl = `https://dns.google/resolve?name=_mcp.${domain}&type=TXT`;
    
    const response = await fetch(dnsQueryUrl);
    const data = await response.json();
    
    if (!data.Answer || data.Answer.length === 0) {
      console.log(`No MCP TXT record found for _mcp.${domain}`);
      return null;
    }
    
    // Extract TXT record value
    let txtRecord = null;
    for (const answer of data.Answer) {
      if (answer.type === 16) { // TXT record type
        txtRecord = answer.data.replace(/"/g, '');
        break;
      }
    }
    
    if (!txtRecord) {
      console.log('No valid TXT record found');
      return null;
    }
    
    console.log('Found TXT record:', txtRecord);
    
    // Parse the TXT record
    return parseMcpTxtRecord(txtRecord);
  } catch (error) {
    console.error('DNS query error:', error);
    return null;
  }
}

// Parse MCP TXT record to extract endpoint and version
function parseMcpTxtRecord(txtRecord) {
  // Extract version (defaults to mcp1)
  const versionMatch = txtRecord.match(/v=([^\s]+)/);
  const version = versionMatch ? versionMatch[1] : 'mcp1';
  
  // Extract discovery URL
  const urlMatch = txtRecord.match(/url=([^\s]+)/);
  if (!urlMatch) {
    console.error('No endpoint URL found in TXT record');
    return null;
  }
  
  const discoveryUrl = urlMatch[1];
  
  // Validate URL
  try {
    new URL(discoveryUrl);
  } catch (e) {
    console.error(`Invalid URL format in TXT record: ${discoveryUrl}`);
    return null;
  }
  
  return {
    version,
    discoveryUrl
  };
}

// Derive WebSocket URL from discovery URL
function deriveWebsocketUrl(discoveryUrl) {
  let url = discoveryUrl;
  
  if (url.startsWith('https://')) {
    url = url.replace('https://', 'wss://');
  } else if (url.startsWith('http://')) {
    url = url.replace('http://', 'ws://');
  }
  
  url = url.replace('/mcpi/discover', '/mcpi');
  
  return url;
}

// Fetch server details from discovery endpoint
async function fetchServerDetails(discoveryUrl) {
  try {
    const response = await fetch(discoveryUrl);
    return await response.json();
  } catch (error) {
    console.error('Error fetching server details:', error);
    return null;
  }
}

// Get current connection status
function getConnectionStatus() {
  const connection = websocketInfo.connection;
  
  if (!connection) {
    return { connected: false };
  }
  
  return {
    connected: connection.readyState === WebSocket.OPEN,
    readyState: connection.readyState,
    initialized: websocketInfo.initialized,
    lastActivity: websocketInfo.lastActivity
  };
}

// Connect to MCPI server
function connectToMcpiServer(websocketUrl) {
  if (!websocketUrl) {
    console.error('No WebSocket URL available');
    return false;
  }
  
  // Clean up any existing connection before creating a new one
  if (websocketInfo.connection) {
    cleanupWebSocketResources();
  }

  try {
    console.log(`Connecting to ${websocketUrl}...`);
    const socket = new WebSocket(websocketUrl);
    
    // Set connection timeout
    const connectionTimeout = setTimeout(() => {
      if (socket.readyState !== WebSocket.OPEN) {
        console.log('Connection timed out');
        socket.close();
      }
    }, 10000); // 10 seconds connection timeout
    
    websocketInfo = {
      connection: socket,
      initialized: false,
      connectionTimeout: connectionTimeout,
      reconnectAttempts: 0,
      lastActivity: Date.now(),
      jsonRpcId: 1,
      discoveredTools: [],
      discoveredResources: []
    };

    // WebSocket event handlers
    socket.onopen = function(event) {
      console.log('WebSocket connection established');
      
      // Clear connection timeout
      if (websocketInfo.connectionTimeout) {
        clearTimeout(websocketInfo.connectionTimeout);
      }
      
      // Send initialize request
      sendJsonRpcRequest('initialize', {
        clientInfo: {
          name: 'MCPI Chrome Extension',
          version: '1.0.0'
        },
        protocolVersion: '0.1.0',
        capabilities: {
          sampling: {}
        }
      });
      
      // Set up ping interval to keep connection alive
      websocketInfo.pingInterval = setInterval(function() {
        // Only send ping if socket is still open
        if (socket.readyState === WebSocket.OPEN) {
          console.log('Sending keepalive ping...');
          sendJsonRpcRequest('ping');
        } else {
          clearInterval(websocketInfo.pingInterval);
        }
      }, 30000); // Send ping every 30 seconds
      
      // Request list of tools after connection
      setTimeout(() => {
        if (socket.readyState === WebSocket.OPEN) {
          // Get list of available tools
          sendJsonRpcRequest('tools/list');
          
          // Get list of available resources
          sendJsonRpcRequest('resources/list');
        }
      }, 500); // Small delay to ensure initialize completes
      
      // Broadcast connection status change
      broadcastConnectionStatusChange();
      
      return true;
    };
    
    socket.onmessage = function(event) {
      // Reset inactivity timer on any message received
      websocketInfo.lastActivity = Date.now();
      
      if (websocketInfo.inactivityTimeout) {
        clearTimeout(websocketInfo.inactivityTimeout);
      }
      
      // Set new inactivity timeout
      websocketInfo.inactivityTimeout = setTimeout(function() {
        console.log('Connection inactive for too long, reconnecting...');
        if (socket.readyState === WebSocket.OPEN) {
          socket.close();
        }
      }, 120000); // 2 minutes of inactivity
      
      handleWebSocketMessage(event.data);
    };
    
    socket.onerror = function(error) {
      console.error('WebSocket error:', error);
      cleanupWebSocketResources();
      broadcastConnectionStatusChange();
    };
    
    socket.onclose = function(event) {
      console.log('WebSocket connection closed:', event.code, event.reason);
      cleanupWebSocketResources();
      broadcastConnectionStatusChange();
    };
    
    return true;
  } catch (error) {
    console.error('WebSocket connection error:', error);
    return false;
  }
}

// Clean up WebSocket resources
function cleanupWebSocketResources() {
  if (!websocketInfo.connection) return;
  
  // Clear all timers
  if (websocketInfo.connectionTimeout) {
    clearTimeout(websocketInfo.connectionTimeout);
  }
  
  if (websocketInfo.pingInterval) {
    clearInterval(websocketInfo.pingInterval);
  }
  
  if (websocketInfo.inactivityTimeout) {
    clearTimeout(websocketInfo.inactivityTimeout);
  }
  
  // Close socket if it's open
  const socket = websocketInfo.connection;
  if (socket && 
      socket.readyState !== WebSocket.CLOSED &&
      socket.readyState !== WebSocket.CLOSING) {
    try {
      socket.close();
    } catch (e) {
      console.error('Error closing WebSocket:', e);
    }
  }
  
  // Keep discovered tools and resources
  const discoveredTools = websocketInfo.discoveredTools;
  const discoveredResources = websocketInfo.discoveredResources;
  
  // Reset connection state
  websocketInfo.connection = null;
  websocketInfo.initialized = false;
  websocketInfo.discoveredTools = discoveredTools;
  websocketInfo.discoveredResources = discoveredResources;
}

// Handle incoming WebSocket message
function handleWebSocketMessage(data) {
  try {
    const message = JSON.parse(data);
    console.log("Received WebSocket message:", message);
    
    // Check for error response
    if (message.error) {
      console.error('JSON-RPC error:', message.error);
      return;
    }
    
    // Handle successful responses
    if (message.result) {
      // Handle initialize response
      if (message.result.serverInfo) {
        websocketInfo.initialized = true;
        broadcastConnectionStatusChange();
      }
      
      // Handle tools/list response
      if (message.result.tools) {
        websocketInfo.discoveredTools = message.result.tools;
        console.log('Discovered tools:', websocketInfo.discoveredTools);
      }
      
      // Handle resources/list response
      if (message.result.resources) {
        websocketInfo.discoveredResources = message.result.resources;
        console.log('Discovered resources:', websocketInfo.discoveredResources);
      }
    }
    
    // Broadcast message to any listening popups
    chrome.runtime.sendMessage({
      action: 'websocketMessage',
      data: message
    }).catch(error => {
      // Only log if it's not the expected "receiving end" error
      if (!error.message.includes("receiving end does not exist")) {
        console.error("Error broadcasting message:", error);
      }
    });
  } catch (error) {
    console.error('Error parsing WebSocket message:', error);
  }
}

// Send JSON-RPC request
function sendJsonRpcRequest(method, params = null) {
  if (!websocketInfo.connection) {
    console.error('No active WebSocket connection');
    return null;
  }
  
  // Only proceed if connection is open
  if (websocketInfo.connection.readyState !== WebSocket.OPEN) {
    console.error('WebSocket connection not open (state:', websocketInfo.connection.readyState, ')');
    return null;
  }
  
  const requestId = websocketInfo.jsonRpcId++;
  
  const request = {
    jsonrpc: '2.0',
    id: requestId,
    method: method
  };
  
  if (params !== null) {
    request.params = params;
  }
  
  try {
    const requestStr = JSON.stringify(request);
    console.log(`Sending request: ${requestStr}`);
    websocketInfo.connection.send(requestStr);
    websocketInfo.lastActivity = Date.now();
    return requestId;
  } catch (error) {
    console.error('Error sending WebSocket message:', error);
    return null;
  }
}

// Broadcast connection status change to any listening popups
function broadcastConnectionStatusChange() {
  chrome.runtime.sendMessage({
    action: 'connectionStatusChanged',
    status: getConnectionStatus()
  }).catch(error => {
    // Only log if it's not the expected "receiving end" error
    if (!error.message.includes("receiving end does not exist")) {
      console.error("Error broadcasting status change:", error);
    }
  });
}

console.log("MCPI Client background script loaded successfully");