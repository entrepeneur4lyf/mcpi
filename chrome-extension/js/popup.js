document.addEventListener('DOMContentLoaded', function() {
  // Panel elements
  const noMcpiPanel = document.getElementById('no-mcpi-panel');
  const connectingPanel = document.getElementById('connecting-panel');
  const mcpiConnectedPanel = document.getElementById('mcpi-connected-panel');
  const toolExecutionPanel = document.getElementById('tool-execution-panel');

  // Server info elements
  const providerName = document.getElementById('provider-name');
  const providerDomain = document.getElementById('provider-domain');
  const providerDescription = document.getElementById('provider-description');

  // Tab elements
  const tabButtons = document.querySelectorAll('.tab-button');
  const tabPanels = document.querySelectorAll('.tab-panel');

  // List containers
  const capabilitiesList = document.getElementById('capabilities-list');
  const toolsList = document.getElementById('tools-list');
  const resourcesList = document.getElementById('resources-list');
  const referralsList = document.getElementById('referrals-list');

  // Button elements
  const backBtn = document.getElementById('back-btn');
  const executeBtn = document.getElementById('execute-btn');
  const reconnectBtn = document.getElementById('reconnect-btn');

  // Tool execution elements
  const toolName = document.getElementById('tool-name');
  const toolDescription = document.getElementById('tool-description');
  const operationSelect = document.getElementById('operation-select');
  const paramsContainer = document.getElementById('params-container');
  const resultContainer = document.getElementById('result-container');
  const resultOutput = document.getElementById('result-output');

  // Global state
  let currentTabId = null;
  let mcpiState = null;
  let toolsData = null;
  let resourcesData = null;
  let currentTool = null;
  let websocketConnection = null;
  let jsonRpcId = 1;

  // Cache for lazy-loaded entity data
  let entityCache = {};

  // Initialize by checking the current tab's MCPI state
  initializeExtension();
  
  // Set up heartbeat to keep connection alive
  let heartbeatInterval;
  let connectionCheckInterval;

  // Button click handlers
  backBtn.addEventListener('click', showConnectedPanel);
  executeBtn.addEventListener('click', executeCurrentTool);
  if (reconnectBtn) {
    reconnectBtn.addEventListener('click', function() {
      showConnectingPanel();
      connectToMcpiServer();
    });
  }

  // Tab navigation
  tabButtons.forEach(button => {
    button.addEventListener('click', () => switchTab(button.id.replace('tab-', '')));
  });

  // Initialize extension
  async function initializeExtension() {
    // Get current tab info
    const tabs = await chrome.tabs.query({ active: true, currentWindow: true });
    if (!tabs || tabs.length === 0) {
      showNoMcpiPanel();
      return;
    }

    currentTabId = tabs[0].id;

    // Show connecting state immediately
    showConnectingPanel();

    // Check MCPI state from background script
    chrome.runtime.sendMessage({ action: 'getMcpiState' }, (response) => {
      mcpiState = response;

      if (mcpiState && mcpiState.supported) {
        // MCPI is supported on this site - check if already connected
        if (mcpiState.connectionStatus && mcpiState.connectionStatus.connected && mcpiState.connectionStatus.initialized) {
          // Already connected - get data and show connected UI
          updateServerInfo();
          // Use discoveredTools from response if available
          if (mcpiState.discoveredTools && mcpiState.discoveredTools.length > 0) {
            handleToolsResult(mcpiState.discoveredTools);
          } else {
            requestTools();
          }
          
          // Use discoveredResources from response if available
          if (mcpiState.discoveredResources && mcpiState.discoveredResources.length > 0) {
            handleResourcesResult(mcpiState.discoveredResources);
          } else {
            requestResources();
          }
          
          showConnectedPanel();
        } else {
          // Not connected or not initialized yet
          connectToMcpiServer();
        }
      } else {
        // No MCPI support on this site
        showNoMcpiPanel();
      }
    });
  }

  // Receive messages from background script
  chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
    if (message.action === 'websocketMessage') {
      handleWebSocketMessage(message.data);
    } else if (message.action === 'connectionStatusChanged') {
      if (message.status.connected && message.status.initialized) {
        updateServerInfo();
        requestTools();
        requestResources();
        showConnectedPanel();
      } else if (!message.status.connected) {
        showDisconnectedState();
      }
    }
  });

  // Show "No MCPI" panel
  function showNoMcpiPanel() {
    noMcpiPanel.style.display = 'block';
    connectingPanel.style.display = 'none';
    mcpiConnectedPanel.style.display = 'none';
    toolExecutionPanel.style.display = 'none';
  }

  // Show "Connecting" panel
  function showConnectingPanel() {
    noMcpiPanel.style.display = 'none';
    connectingPanel.style.display = 'block';
    mcpiConnectedPanel.style.display = 'none';
    toolExecutionPanel.style.display = 'none';
  }

  // Show "Connected" panel
  function showConnectedPanel() {
    noMcpiPanel.style.display = 'none';
    connectingPanel.style.display = 'none';
    mcpiConnectedPanel.style.display = 'block';
    toolExecutionPanel.style.display = 'none';

    // Default to capabilities tab
    switchTab('capabilities');
  }

  // Show tool execution panel
  function showToolExecutionPanel() {
    noMcpiPanel.style.display = 'none';
    connectingPanel.style.display = 'none';
    mcpiConnectedPanel.style.display = 'none';
    toolExecutionPanel.style.display = 'block';
  }

  // Show disconnected state
  function showDisconnectedState() {
    // Create reconnect notification if it doesn't exist
    let reconnectNotice = document.getElementById('reconnect-notice');
    if (!reconnectNotice) {
      reconnectNotice = document.createElement('div');
      reconnectNotice.id = 'reconnect-notice';
      reconnectNotice.className = 'disconnect-notice';
      reconnectNotice.innerHTML = `
        <div class="status-badge error">Disconnected</div>
        <p>Connection to MCPI server was lost.</p>
        <button id="reconnect-now-btn" class="btn primary">Reconnect</button>
      `;
      
      // Insert at the top of whatever panel is visible
      if (mcpiConnectedPanel.style.display !== 'none') {
        mcpiConnectedPanel.insertBefore(reconnectNotice, mcpiConnectedPanel.firstChild);
      } else if (toolExecutionPanel.style.display !== 'none') {
        toolExecutionPanel.insertBefore(reconnectNotice, toolExecutionPanel.firstChild);
      }
      
      // Add event listener to the reconnect button
      document.getElementById('reconnect-now-btn').addEventListener('click', function() {
        // Remove reconnect notice
        reconnectNotice.remove();
        
        // Show connecting panel
        showConnectingPanel();
        
        // Try to reconnect
        connectToMcpiServer();
      });
    }
  }

  // Switch between tabs
  function switchTab(tabName) {
    // Update tab buttons
    tabButtons.forEach(button => {
      if (button.id === `tab-${tabName}`) {
        button.classList.add('active');
      } else {
        button.classList.remove('active');
      }
    });

    // Update tab panels
    tabPanels.forEach(panel => {
      if (panel.id === `panel-${tabName}`) {
        panel.classList.add('active');
      } else {
        panel.classList.remove('active');
      }
    });
  }

  // Connect to MCPI server
  function connectToMcpiServer() {
    if (!mcpiState || !mcpiState.websocketUrl) {
      console.error('No WebSocket URL available');
      showNoMcpiPanel();
      return;
    }
    
    // For simplicity, we'll use the background script to manage connections
    chrome.runtime.sendMessage({
      action: 'connectToMcpi',
      tabId: currentTabId
    }, function(response) {
      if (!response || !response.success) {
        console.error('Failed to connect to MCPI server');
        showNoMcpiPanel();
      }
      // We'll let the message listener handle the rest
    });
  }
  
  // Update server info
  function updateServerInfo() {
    if (mcpiState && mcpiState.serverDetails && mcpiState.serverDetails.provider) {
      providerName.textContent = mcpiState.serverDetails.provider.name || 'Unknown Server';
      providerDomain.textContent = mcpiState.serverDetails.provider.domain || '';
      providerDescription.textContent = mcpiState.serverDetails.provider.description || 'No description available';
      
      // Update capabilities if available
      if (mcpiState.serverDetails.capabilities) {
        updateCapabilities(mcpiState.serverDetails.capabilities);
      }
      
      // Update referrals if available
      if (mcpiState.serverDetails.referrals) {
        updateReferrals(mcpiState.serverDetails.referrals);
      }
    }
  }

  // Update capabilities listing
  function updateCapabilities(capabilities) {
    capabilitiesList.innerHTML = '';
    
    if (!capabilities || capabilities.length === 0) {
      capabilitiesList.innerHTML = '<p style="padding: 10px; color: var(--neutral-color);">No capabilities available.</p>';
      return;
    }
    
    capabilities.forEach(capability => {
      const item = document.createElement('div');
      item.className = 'list-item';
      item.innerHTML = `
        <h3>${capability.name}</h3>
        <p>${capability.description}</p>
        <div class="meta">
          <span class="category">${capability.category}</span>
          <span class="operations">${capability.operations.join(', ')}</span>
        </div>
      `;
      
      capabilitiesList.appendChild(item);
    });
  }
  
  // Update referrals listing
  function updateReferrals(referrals) {
    referralsList.innerHTML = '';
    
    if (!referrals || referrals.length === 0) {
      referralsList.innerHTML = '<p style="padding: 10px; color: var(--neutral-color);">No referrals available.</p>';
      return;
    }
    
    referrals.forEach(referral => {
      const item = document.createElement('div');
      item.className = 'list-item';
      
      const relationshipClass = `referral-${referral.relationship}`;
      
      item.innerHTML = `
        <h3>${referral.name}</h3>
        <div class="meta">
          <span>${referral.domain}</span>
          <span class="referral-relationship ${relationshipClass}">${referral.relationship}</span>
        </div>
      `;
      
      referralsList.appendChild(item);
    });
  }
  
  // Request tools
  function requestTools() {
    chrome.runtime.sendMessage({
      action: 'sendRequest',
      method: 'tools/list',
      params: {}
    });
  }
  
  // Request resources
  function requestResources() {
    chrome.runtime.sendMessage({
      action: 'sendRequest',
      method: 'resources/list',
      params: {}
    });
  }

  // Handle incoming WebSocket message
  function handleWebSocketMessage(message) {
    // Process the message based on content
    if (message.result) {
      // Handle initialize response
      if (message.result.serverInfo) {
        // We're connected and initialized
        showConnectedPanel();
      }
      
      // Handle tools/list response
      if (message.result.tools) {
        handleToolsResult(message.result.tools);
      }
      
      // Handle resources/list response
      if (message.result.resources) {
        handleResourcesResult(message.result.resources);
      }
      
      // Handle tool execution result
      if (message.result.content) {
        handleToolExecutionResult(message.result);
      }
    }
  }
  
  // Function to lazily fetch entity data when needed
  async function getEntityData(toolName, operation) {
    // Create a cache key
    const cacheKey = `${toolName}_${operation}`;
    
    // Return cached data if available
    if (entityCache[cacheKey]) {
      return entityCache[cacheKey];
    }
    
    // Otherwise fetch the data
    return new Promise((resolve, reject) => {
      chrome.runtime.sendMessage({
        action: 'sendRequest',
        method: 'tools/call',
        params: {
          name: toolName,
          arguments: {
            operation: operation
          }
        }
      }, function(response) {
        if (!response || !response.success) {
          reject(new Error('Failed to send request'));
          return;
        }
        
        // Store request ID to match with response
        const requestId = response.requestId;
        
        // Set up a listener for this specific response
        const messageListener = function(message) {
          if (message.action === 'websocketMessage' && 
              message.data && 
              message.data.id === requestId) {
            
            // We got our response, process it
            if (message.data.result && message.data.result.content) {
              const contentText = message.data.result.content[0]?.text;
              if (contentText) {
                try {
                  const data = JSON.parse(contentText);
                  // Cache the results
                  entityCache[cacheKey] = data.results || [];
                  resolve(entityCache[cacheKey]);
                } catch (e) {
                  console.error('Error parsing response:', e);
                  reject(e);
                }
              } else {
                reject(new Error('No content in response'));
              }
            } else {
              reject(new Error('Invalid response format'));
            }
            
            // Remove the listener
            chrome.runtime.onMessage.removeListener(messageListener);
          }
        };
        
        // Add the listener
        chrome.runtime.onMessage.addListener(messageListener);
      });
    });
  }

// Generate appropriate test parameters for a quick action
async function generateTestParams(tool, operation) {
  // Default params object
  const params = {};
  
  // Different parameters based on tool and operation type
  if (operation === "HELLO") {
    params.context = "general";
  }
  else if (operation.includes("GET") || operation === "GET") {
    // For GET operations, we need to find a real ID
    // First determine which entity type we're working with
    let listOp = "LIST";
    let idField = "id";
    
    if (tool.name === "store_product") {
      listOp = "LIST_PRODUCTS";
    } else if (tool.name === "store_customer") {
      listOp = "LIST_CUSTOMERS";
    } else if (tool.name === "store_order") {
      listOp = "LIST_ORDERS";
    } else if (tool.name === "store_review") {
      listOp = "LIST_REVIEWS";
    } else if (tool.name === "weather_forecast") {
      // Special case for weather forecast - it needs a location parameter
      params.location = "New York";
      return params; // Return early with location parameter
    }
    
    // Fetch entity data if needed
    try {
      const entities = await getEntityData(tool.name, listOp);
      
      if (entities && entities.length > 0) {
        // Get the first entity's ID
        params.id = entities[0].id;
      } else {
        // Fallback to some default IDs if no entities found
        if (tool.name === "store_product") params.id = "eco-1001";
        else if (tool.name === "store_customer") params.id = "cust-1001";
        else if (tool.name === "store_order") params.id = "order-5001";
        else if (tool.name === "store_review") params.id = "rev-2001";
        else if (tool.name === "website") params.id = "about";
        else params.id = "sample-id";
      }
    } catch (error) {
      console.error(`Error fetching data for ${tool.name}:`, error);
      // Fallback to some default IDs
      if (tool.name === "store_product") params.id = "eco-1001";
      else if (tool.name === "store_customer") params.id = "cust-1001";
      else if (tool.name === "store_order") params.id = "order-5001";
      else if (tool.name === "store_review") params.id = "rev-2001";
      else if (tool.name === "website") params.id = "about";
      else params.id = "sample-id";
    }
  }
  else if (operation.includes("SEARCH") || operation === "SEARCH") {
    // For search operations, use reasonable defaults
    params.query = "test";
    
    // Use appropriate field names based on the entity
    if (tool.name === "store_product") params.field = "name";
    else if (tool.name === "store_customer") params.field = "name";
    else if (tool.name === "store_review") params.field = "content";
    else if (tool.name === "website") params.field = "title";
    else params.field = "name";
  }
  
  return params;
}

// Generate a meaningful label for a quick action
async function generateActionLabel(tool, operation, params) {
  let label = operation;
  
  // Make labels more user-friendly
  if (operation === "HELLO") {
    return "Get site introduction";
  }
  else if (operation.includes("LIST") || operation === "LIST") {
    return `List all ${operation.replace("LIST_", "").toLowerCase()}`;
  }
  else if (operation.includes("SEARCH") || operation === "SEARCH") {
    return `Search ${operation.replace("SEARCH_", "").toLowerCase()}`;
  }
  else if (operation.includes("GET") || operation === "GET") {
    // For GET operations, try to include the entity name
    const id = params.id;
    
    // Default labels with prefix
    if (tool.name === "store_product") return `View product: ${id}`;
    else if (tool.name === "store_customer") return `View customer: ${id}`;
    else if (tool.name === "store_order") return `View order: ${id}`;
    else if (tool.name === "store_review") return `View review: ${id}`;
    else if (tool.name === "website" && id === "about") return `View page: About`;
    else return `View item: ${id}`;
  }
  
  // Default case
  return `${operation}`;
}

// Create dynamic quick actions for a tool
async function createDynamicQuickActions(tool) {
  const quickActions = [];
  
  // Get supported operations from the tool's schema
  const operations = extractOperationsFromSchema(tool.inputSchema);
  
  // Function to map generic LIST operation to tool-specific LIST operations
  function mapListOperation(toolName) {
    // Map generic LIST to tool-specific LIST operations
    if (toolName === 'social') return 'LIST_REFERRALS';
    if (toolName === 'store_product') return 'LIST_PRODUCTS';
    if (toolName === 'store_customer') return 'LIST_CUSTOMERS';
    if (toolName === 'store_order') return 'LIST_ORDERS';
    if (toolName === 'store_review') return 'LIST_REVIEWS';
    // Default fallback
    return 'LIST';
  }
  
  // 1. LIST operations (always first and simplest)
  const listOps = operations.filter(op => op.includes('LIST') || op === 'LIST');
  for (const operation of listOps) {
    // Use the mapped operation but keep the original display name for the label
    const mappedOperation = operation === 'LIST' ? mapListOperation(tool.name) : operation;
    
    quickActions.push({ 
      operation: mappedOperation,
      label: `List all ${operation.replace("LIST_", "").toLowerCase()}`, 
      params: {},
      type: "discovery" // Mark as a discovery operation 
    });
  }
  
  // 2. SEARCH operations
  const searchOps = operations.filter(op => op.includes('SEARCH') || op === 'SEARCH');
  for (const operation of searchOps) {
    quickActions.push({ 
      operation, 
      label: `Search ${operation.replace("SEARCH_", "").toLowerCase()}`, 
      params: { query: "test", field: "name" },
      type: "discovery" // Mark as a discovery operation
    });
  }
  
  // 3. HELLO operations
  if (operations.includes('HELLO')) {
    quickActions.push({ 
      operation: "HELLO", 
      label: "Get site introduction", 
      params: { context: "general" },
      type: "discovery" // Mark as a discovery operation
    });
  }
  
  // 4. GET operations (as testing tools)
  const getOps = operations.filter(op => op.includes('GET') || op === 'GET');
  for (const operation of getOps) {
    // Generate appropriate parameters for this operation
    const params = await generateTestParams(tool, operation);
    
    // Generate a user-friendly label that clearly indicates it's a test
    const label = await generateActionLabel(tool, operation, params);
    
    quickActions.push({ 
      operation, 
      label: `${label}`, // Make label clearer
      params,
      type: "test" // Mark as a test operation
    });
  }
  
  return quickActions;
}
  
  // Extract operations from a tool's input schema
  function extractOperationsFromSchema(schema) {
    // Default operations if none found
    if (!schema) return ["SEARCH", "GET", "LIST"];
    
    // Try to extract from schema
    return schema.properties?.operation?.enum || ["SEARCH", "GET", "LIST"];
  }
  
  // Handle tools result
  function handleToolsResult(tools) {
    toolsData = tools;
    
    // Call the async update function
    updateToolsList(tools);
  }
  
  // Update the function that adds tools to the list
  async function updateToolsList(tools) {
    toolsList.innerHTML = '';
    
    for (const tool of tools) {
      const item = document.createElement('div');
      item.className = 'list-item tool-item';
      
      // Extract operations from input schema
      const operations = extractOperationsFromSchema(tool.inputSchema);
      
      let toolHtml = `
        <h3>${tool.name}</h3>
        <p>${tool.description || 'No description available'}</p>
        <div class="meta">
          <span class="operations">${operations.join(', ')}</span>
        </div>`;
      
      // Add loading indicator for quick actions
      toolHtml += `<div class="quick-actions" id="quick-actions-${tool.name}">`;
      toolHtml += '<span class="loading">Loading actions...</span>';
      toolHtml += '</div>';
      
      item.innerHTML = toolHtml;
      
      // Add click handler to open tool execution panel
      item.querySelector('h3').addEventListener('click', function() {
        openToolExecutionPanel(tool);
      });
      
      toolsList.appendChild(item);
      
      // Now asynchronously create quick actions
      const quickActions = await createDynamicQuickActions(tool);
      
      // Replace loading indicator with quick actions
      const quickActionsContainer = item.querySelector('.quick-actions');
      quickActionsContainer.innerHTML = '';
      
      // First add API operation buttons (discovery operations)
      const discoveryActions = quickActions.filter(action => action.type === "discovery");
      if (discoveryActions.length > 0) {
        const apiSection = document.createElement('div');
        apiSection.className = 'action-section';
        apiSection.innerHTML = '<h4>API Operations</h4>';
        
        const buttonsContainer = document.createElement('div');
        buttonsContainer.className = 'buttons-wrapper';
        
        discoveryActions.forEach(action => {
          const button = document.createElement('button');
          button.className = 'quick-action-btn';
          button.setAttribute('data-tool', tool.name);
          button.setAttribute('data-operation', action.operation);
          button.setAttribute('data-params', JSON.stringify(action.params));
          button.textContent = action.label;
          
          // Add click handler
          button.addEventListener('click', function(e) {
            e.stopPropagation();
            
            const toolName = this.getAttribute('data-tool');
            const operation = this.getAttribute('data-operation');
            const params = JSON.parse(this.getAttribute('data-params'));
            
            // Find the tool object
            const tool = tools.find(t => t.name === toolName);
            if (tool) {
              executeQuickAction(tool, operation, params);
            }
          });
          
          buttonsContainer.appendChild(button);
        });
        
        apiSection.appendChild(buttonsContainer);
        quickActionsContainer.appendChild(apiSection);
      }
      
      // Then add test operation buttons
      const testActions = quickActions.filter(action => action.type === "test");
      if (testActions.length > 0) {
        const testSection = document.createElement('div');
        testSection.className = 'action-section test-actions';
        testSection.innerHTML = '<h4>Test Operations</h4>';
        
        const buttonsContainer = document.createElement('div');
        buttonsContainer.className = 'buttons-wrapper';
        
        testActions.forEach(action => {
          const button = document.createElement('button');
          button.className = 'quick-action-btn test-btn';
          button.setAttribute('data-tool', tool.name);
          button.setAttribute('data-operation', action.operation);
          button.setAttribute('data-params', JSON.stringify(action.params));
          button.textContent = action.label;
          button.title = action.label; // Add title for hover text on overflow
          
          // Add click handler
          button.addEventListener('click', function(e) {
            e.stopPropagation();
            
            const toolName = this.getAttribute('data-tool');
            const operation = this.getAttribute('data-operation');
            const params = JSON.parse(this.getAttribute('data-params'));
            
            // Find the tool object
            const tool = tools.find(t => t.name === toolName);
            if (tool) {
              executeQuickAction(tool, operation, params);
            }
          });
          
          buttonsContainer.appendChild(button);
        });
        
        testSection.appendChild(buttonsContainer);
        quickActionsContainer.appendChild(testSection);
      }
      
      if (quickActions.length === 0) {
        quickActionsContainer.innerHTML = '<span>No actions available</span>';
      }
    }
  }
  
  // Execute a quick action
  function executeQuickAction(tool, operation, params) {
    // Show tool execution panel with pre-filled values
    openToolExecutionPanel(tool, operation, params);
    
    // Auto-execute the tool
    executeCurrentTool();
  }
  
  // Handle resources result
  function handleResourcesResult(resources) {
    resourcesData = resources;
    
    resourcesList.innerHTML = '';
    
    resources.forEach(resource => {
      const item = document.createElement('div');
      item.className = 'list-item';
      
      item.innerHTML = `
        <h3>${resource.name}</h3>
        <p>${resource.description || 'No description available'}</p>
        <div class="meta">
          <span>${resource.uri}</span>
          <span>${resource.mimeType || 'unknown'}</span>
        </div>
      `;
      
      resourcesList.appendChild(item);
    });
  }
  
  // Open tool execution panel
  function openToolExecutionPanel(tool, preselectedOperation = null, prefillParams = null) {
    currentTool = tool;
    
    // Update tool info
    toolName.textContent = tool.name;
    toolDescription.textContent = tool.description || 'No description available';
    
    // Clear previous results
    resultContainer.style.display = 'none';
    
    // Set available operations
    operationSelect.innerHTML = '';
    
    // Extract operations from schema
    const operations = extractOperationsFromSchema(tool.inputSchema);
    
    operations.forEach(op => {
      const option = document.createElement('option');
      option.value = op;
      option.textContent = op;
      if (preselectedOperation && op === preselectedOperation) {
        option.selected = true;
      }
      operationSelect.appendChild(option);
    });
    
    // Build parameter inputs based on schema
    generateParamInputs(prefillParams);
    
    // Remove old event listener and add new one
    operationSelect.removeEventListener('change', generateParamInputs);
    operationSelect.addEventListener('change', generateParamInputs);
    
    // Show tool execution panel
    showToolExecutionPanel();
  }
  
  // Generate parameter inputs based on schema and selected operation
  function generateParamInputs(prefillParams = null) {
    paramsContainer.innerHTML = '';
    
    if (!currentTool || !currentTool.inputSchema || !currentTool.inputSchema.properties) {
      return;
    }
    
    const schema = currentTool.inputSchema;
    const operation = operationSelect.value;
    
    // Add inputs for each property in the schema
    for (const [key, prop] of Object.entries(schema.properties)) {
      // Skip operation field as it's handled by the select
      if (key === 'operation') {
        continue;
      }
      
      // Create parameter group
      const paramGroup = document.createElement('div');
      paramGroup.className = 'param-group';
      
      // Create label
      const label = document.createElement('label');
      label.htmlFor = `param-${key}`;
      label.textContent = `${key}${prop.description ? ' - ' + prop.description : ''}`;
      
      // Create input
      const input = document.createElement('input');
      input.type = 'text';
      input.id = `param-${key}`;
      input.name = key;
      
      // Add placeholder based on parameter type and operation
      input.placeholder = getPlaceholderForParam(key, operation, currentTool.name);
      
      // Set value from prefill params if available
      if (prefillParams && prefillParams[key] !== undefined) {
        input.value = prefillParams[key];
      }
      
      // Add to DOM
      paramGroup.appendChild(label);
      paramGroup.appendChild(input);
      paramsContainer.appendChild(paramGroup);
    }
  }
  
// Get placeholder text for parameter input based on context
function getPlaceholderForParam(paramName, operation, toolName) {
  // General placeholders based on parameter name
  if (paramName === 'id') {
    if (toolName === 'store_product') 
      return 'Enter product ID (e.g., eco-1001)';
    else if (toolName === 'store_customer')
      return 'Enter customer ID (e.g., cust-1001)';
    else if (toolName === 'store_order')
      return 'Enter order ID (e.g., order-5001)';
    else if (toolName === 'store_review')
      return 'Enter review ID (e.g., rev-2001)';
    else if (toolName === 'website')
      return 'Enter content ID (e.g., about)';
    else
      return 'Enter ID';
  }
  
  if (paramName === 'query') {
    return 'Enter search query';
  }
  
  if (paramName === 'field') {
    return 'Field to search in (e.g., name)';
  }
  
  if (paramName === 'context' && operation === 'HELLO') {
    return 'Context like "shopping" or "support"';
  }
  
  if (paramName === 'detail_level' && operation === 'HELLO') {
    return 'basic, standard, or detailed';
  }
  
  // Default placeholder
  return `Enter ${paramName}`;
}
  
  // Execute current tool
  function executeCurrentTool() {
    if (!currentTool) {
      return;
    }
    
    // Get selected operation
    const operation = operationSelect.value;
    
    // Collect parameter values
    const arguments = {
      operation: operation
    };
    
    // Add all other parameters
    const paramInputs = paramsContainer.querySelectorAll('input');
    paramInputs.forEach(input => {
      if (input.value.trim()) {
        arguments[input.name] = input.value.trim();
      }
    });
    
    // Show loading state
    resultContainer.style.display = 'block';
    resultOutput.textContent = 'Executing...';
    
    // Send tool call request
    chrome.runtime.sendMessage({
      action: 'sendRequest',
      method: 'tools/call',
      params: {
        name: currentTool.name,
        arguments: arguments
      }
    }, function(response) {
      if (!response || !response.success) {
        resultOutput.textContent = 'Error: Failed to execute tool. Connection may be lost.';
        showDisconnectedState();
      }
    });
  }
  
  // Handle tool execution result
  function handleToolExecutionResult(result) {
    if (!result.content || result.content.length === 0) {
      resultOutput.textContent = 'No result returned';
      return;
    }
    
    // Display text content if available
    const textContent = result.content.find(c => c.type === 'text' && c.text);
    
    if (textContent) {
      try {
        // Try to parse and pretty-print JSON
        const parsedJson = JSON.parse(textContent.text);
        resultOutput.textContent = JSON.stringify(parsedJson, null, 2);
      } catch (e) {
        // Not JSON, just show as is
        resultOutput.textContent = textContent.text;
      }
    } else {
      resultOutput.textContent = 'Result did not contain text content';
    }
    
    resultContainer.style.display = 'block';
  }
});