// Background script for MCPI Client extension

// Track MCPI discovery state for each tab
let mcpiTabStates = {};

// Listen for tab updates to detect domain changes
chrome.tabs.onUpdated.addListener((tabId, changeInfo, tab) => {
  // Only run when the tab has completed loading
  if (changeInfo.status === 'complete' && tab.url) {
    checkForMcpiSupport(tab.url, tabId);
  }
});

// Listen for tab activation to update icon if needed
chrome.tabs.onActivated.addListener((activeInfo) => {
  updateIconForTab(activeInfo.tabId);
});

// Listen for messages from popup
chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
  if (message.action === 'getMcpiState') {
    // Get the current tab's MCPI state
    chrome.tabs.query({ active: true, currentWindow: true }, (tabs) => {
      if (tabs[0]) {
        const tabId = tabs[0].tabId || tabs[0].id;
        const state = mcpiTabStates[tabId] || { supported: false };
        sendResponse(state);
      } else {
        sendResponse({ supported: false });
      }
    });
    
    return true; // Return true to indicate async response
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
  chrome.action.setIcon({
    tabId: tabId,
    path: {
      16: "icons/icon_active.svg",
      48: "icons/icon_active.svg",
      128: "icons/icon_active.svg"
    }
  });
  
  // Update badge to show it's available
  chrome.action.setBadgeText({ tabId: tabId, text: "MCP" });
  chrome.action.setBadgeBackgroundColor({ tabId: tabId, color: "#2ecc71" });
}

// Set icon to inactive state (gray)
function setIconInactive(tabId) {
  chrome.action.setIcon({
    tabId: tabId,
    path: {
      16: "icons/icon_inactive.svg",
      48: "icons/icon_inactive.svg",
      128: "icons/icon_inactive.svg"
    }
  });
  
  // Clear badge
  chrome.action.setBadgeText({ tabId: tabId, text: "" });
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