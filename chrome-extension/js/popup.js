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
          requestTools();
          requestResources();
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
  
  // Handle tools result
  function handleToolsResult(tools) {
    toolsData = tools;
    
    toolsList.innerHTML = '';
    
    // Add example quick actions for product_search, customer_lookup, etc.
    const quickActionTools = {
      'product_search': [
        { operation: 'SEARCH', label: 'Search for "bamboo" products', params: { query: 'bamboo' } },
        { operation: 'GET', label: 'Get Bamboo Water Bottle details', params: { id: 'eco-1001' } },
        { operation: 'LIST', label: 'List all products', params: {} }
      ],
      'customer_lookup': [
        { operation: 'GET', label: 'Look up customer Jane Smith', params: { id: 'cust-1001' } },
        { operation: 'LIST', label: 'List all customers', params: {} }
      ],
      'order_history': [
        { operation: 'GET', label: 'View order details', params: { id: 'order-5001' } },
        { operation: 'SEARCH', label: 'Find Jane\'s orders', params: { field: 'customer_id', query: 'cust-1001' } }
      ],
      'product_reviews': [
        { operation: 'SEARCH', label: 'Read reviews for Bamboo Bottle', params: { field: 'product_id', query: 'eco-1001' } }
      ],
      'website_content': [
        { operation: 'GET', label: 'View About page', params: { id: 'about' } },
        { operation: 'SEARCH', label: 'Find sustainability content', params: { query: 'sustainability' } }
      ],
      'weather_forecast': [
        { operation: 'GET', label: 'Get London weather', params: { location: 'London' } }
      ]
    };
    
    tools.forEach(tool => {
      const item = document.createElement('div');
      item.className = 'list-item tool-item';
      
      // Extract operations from input schema if available
      let operations = [];
      if (tool.inputSchema && 
          tool.inputSchema.properties && 
          tool.inputSchema.properties.operation && 
          tool.inputSchema.properties.operation.enum) {
        operations = tool.inputSchema.properties.operation.enum;
      }
      
      let toolHtml = `
        <h3>${tool.name}</h3>
        <p>${tool.description || 'No description available'}</p>
        <div class="meta">
          <span class="operations">${operations.join(', ')}</span>
        </div>`;
      
      // Add quick action buttons if available for this tool
      if (quickActionTools[tool.name]) {
        toolHtml += '<div class="quick-actions">';
        
        quickActionTools[tool.name].forEach(action => {
          toolHtml += `<button class="quick-action-btn" 
            data-tool="${tool.name}" 
            data-operation="${action.operation}" 
            data-params='${JSON.stringify(action.params)}'>
            ${action.label}
          </button>`;
        });
        
        toolHtml += '</div>';
      }
      
      item.innerHTML = toolHtml;
      
      // Add click handler to open tool execution panel
      item.querySelector('h3').addEventListener('click', function() {
        openToolExecutionPanel(tool);
      });
      
      // Add click handlers for quick action buttons
      const quickActionBtns = item.querySelectorAll('.quick-action-btn');
      quickActionBtns.forEach(btn => {
        btn.addEventListener('click', function(e) {
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
      });
      
      toolsList.appendChild(item);
    });
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
    if (tool.inputSchema && 
        tool.inputSchema.properties && 
        tool.inputSchema.properties.operation && 
        tool.inputSchema.properties.operation.enum) {
      
      tool.inputSchema.properties.operation.enum.forEach(op => {
        const option = document.createElement('option');
        option.value = op;
        option.textContent = op;
        if (preselectedOperation && op === preselectedOperation) {
          option.selected = true;
        }
        operationSelect.appendChild(option);
      });
    }
    
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
      switch (operation) {
        case 'GET':
          if (key === 'id') {
            input.placeholder = 'Enter ID (e.g., eco-1001)';
          }
          break;
        case 'SEARCH':
          if (key === 'query') {
            input.placeholder = 'Enter search query';
          } else if (key === 'field') {
            input.placeholder = 'Field to search in (e.g., name)';
          }
          break;
      }
      
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