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
    let websocketConnection = null;
    let toolsData = null;
    let resourcesData = null;
    let currentTool = null;
    let jsonRpcId = 1;
  
    // Initialize by checking the current tab's MCPI state
    initializeExtension();
  
    // Button click handlers
    backBtn.addEventListener('click', showConnectedPanel);
    executeBtn.addEventListener('click', executeCurrentTool);
  
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
          // MCPI is supported on this site - connect automatically
          connectToMcpiServer();
        } else {
          // No MCPI support on this site
          showNoMcpiPanel();
        }
      });
    }
  
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
  
      try {
        const socket = new WebSocket(mcpiState.websocketUrl);
        
        websocketConnection = {
          socket: socket,
          initialized: false
        };
        
        // WebSocket event handlers
        socket.onopen = function(event) {
          console.log('WebSocket connection established');
          
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
        };
        
        socket.onmessage = function(event) {
          handleWebSocketMessage(event.data);
        };
        
        socket.onerror = function(error) {
          console.error('WebSocket error:', error);
          showNoMcpiPanel();
        };
        
        socket.onclose = function(event) {
          console.log('WebSocket connection closed:', event.code, event.reason);
          websocketConnection = null;
          showNoMcpiPanel();
        };
      } catch (error) {
        console.error('WebSocket error:', error);
        showNoMcpiPanel();
      }
    }
    
    // Send JSON-RPC request
    function sendJsonRpcRequest(method, params = null) {
      if (!websocketConnection || !websocketConnection.socket) {
        console.error('No active WebSocket connection');
        return;
      }
      
      const requestId = jsonRpcId++;
      
      const request = {
        jsonrpc: '2.0',
        id: requestId,
        method: method
      };
      
      if (params !== null) {
        request.params = params;
      }
      
      websocketConnection.socket.send(JSON.stringify(request));
      return requestId;
    }
    
    // Handle incoming WebSocket message
    function handleWebSocketMessage(data) {
      try {
        const message = JSON.parse(data);
        
        // Check for error response
        if (message.error) {
          console.error('JSON-RPC error:', message.error);
          if (resultContainer.style.display === 'block') {
            resultOutput.textContent = `Error (code ${message.error.code}): ${message.error.message}`;
          }
          return;
        }
        
        // Handle successful responses
        if (message.result) {
          // Handle initialize response
          if (!websocketConnection.initialized && message.result.serverInfo) {
            handleInitializeResult(message.result);
            return;
          }
          
          // Handle tools/list response
          if (message.result.tools) {
            handleToolsResult(message.result.tools);
            return;
          }
          
          // Handle resources/list response
          if (message.result.resources) {
            handleResourcesResult(message.result.resources);
            return;
          }
          
          // Handle tool execution result
          if (message.result.content) {
            handleToolExecutionResult(message.result);
            return;
          }
        }
      } catch (error) {
        console.error('Error parsing WebSocket message:', error);
      }
    }
    
    // Handle initialize result
    function handleInitializeResult(result) {
      websocketConnection.initialized = true;
      
      console.log('Connected to MCPI server:', result.serverInfo.name, 'v' + result.serverInfo.version);
      
      // Update server info in connected panel
      if (mcpiState && mcpiState.serverDetails) {
        const provider = mcpiState.serverDetails.provider;
        providerName.textContent = provider.name;
        providerDomain.textContent = provider.domain;
        providerDescription.textContent = provider.description;
      }
      
      // Update UI
      showConnectedPanel();
      
      // Populate capabilities from discovery data
      if (mcpiState && mcpiState.serverDetails && mcpiState.serverDetails.capabilities) {
        updateCapabilities(mcpiState.serverDetails.capabilities);
      }
      
      // Populate referrals from discovery data
      if (mcpiState && mcpiState.serverDetails && mcpiState.serverDetails.referrals) {
        updateReferrals(mcpiState.serverDetails.referrals);
      }
      
      // Request available tools
      sendJsonRpcRequest('tools/list');
      
      // Request available resources
      sendJsonRpcRequest('resources/list');
    }
    
    // Update capabilities listing
    function updateCapabilities(capabilities) {
      capabilitiesList.innerHTML = '';
      
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
      
      // Add operation change handler to update parameters
      operationSelect.addEventListener('change', () => generateParamInputs());
      
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
      sendJsonRpcRequest('tools/call', {
        name: currentTool.name,
        arguments: arguments
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