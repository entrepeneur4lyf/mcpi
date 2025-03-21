# Hello Protocol Extension Documentation

## Introduction

The Hello Protocol is an MCPI extension that enables efficient introduction between AI agents and websites. Rather than requiring AI agents to process and understand entire websites—a process that is token-intensive, slow, and often ineffective—the Hello Protocol provides a standardized method for websites to introduce themselves to AI agents in a structured, efficient manner.

## Purpose

The core problems that the Hello Protocol solves:

1. **Efficiency**: Dramatically reduces the token usage and processing time for AI agents to understand what a website offers
2. **Personalization**: Allows websites to tailor their introduction based on visitor context
3. **SEO for AI**: Enables websites to optimize for AI discovery through controlled introduction content
4. **Capability Advertising**: Informs AI agents about available tools/operations immediately

## Protocol Specification

### Operation Details

| Field | Value |
|------|-------|
| Operation | `HELLO` |
| Standard Plugin | `website_content` |
| Required Parameters | None |
| Optional Parameters | `context`, `detail_level` |
| Response Format | Introduction text with metadata |

### Request Format

```json
{
  "jsonrpc": "2.0",
  "id": "request-id",
  "method": "tools/call",
  "params": {
    "name": "website_content",
    "arguments": {
      "operation": "HELLO",
      "context": "optional context about requester's intent",
      "detail_level": "basic|standard|detailed"
    }
  }
}
```

### Response Format

```json
{
  "jsonrpc": "2.0",
  "id": "request-id",
  "result": {
    "content": [
      {
        "type": "text",
        "text": "Hello! I'm the AI assistant for Example Store, an online retailer of eco-friendly products founded in 2020. We specialize in sustainable alternatives to everyday items including bamboo products, recycled materials, and zero-waste options. Our most popular categories are kitchen supplies, personal care, and home goods. How can I assist you today?"
      }
    ],
    "metadata": {
      "provider": {
        "name": "Example Store",
        "domain": "example.com",
        "description": "Online retailer of eco-friendly products"
      },
      "capabilities": [
        "product_search",
        "customer_lookup",
        "order_history"
      ],
      "topics": [
        "sustainability",
        "eco-friendly",
        "zero-waste"
      ],
      "key_offerings": [
        "bamboo products",
        "recycled goods",
        "sustainable alternatives"
      ],
      "unique_selling_points": [
        "carbon-neutral shipping",
        "plastic-free packaging"
      ]
    }
  }
}
```

## Implementation Guide

### Server-Side Implementation

#### 1. Configuration File

Create a `hello_config.json` file in your data directory with customizable introduction templates:

```json
{
  "default": {
    "introduction": "Hello! I'm the AI assistant for Example Store, an online retailer of eco-friendly products. How can I assist you today?",
    "metadata": {
      "primary_focus": ["sustainability", "eco-friendly", "zero-waste"],
      "key_offerings": ["bamboo products", "recycled goods", "sustainable alternatives"],
      "unique_selling_points": ["carbon-neutral shipping", "plastic-free packaging"]
    }
  },
  "contexts": {
    "shopping": {
      "introduction": "Welcome to Example Store! We offer a wide range of eco-friendly products with free shipping on orders over $50. Our most popular categories include kitchen supplies, personal care, and home goods.",
      "highlight_capabilities": ["product_search", "order_history"]
    },
    "business": {
      "introduction": "Example Store provides sustainable product solutions for businesses of all sizes. We offer bulk discounts, corporate gifting, and custom branding options.",
      "highlight_capabilities": ["wholesale_inquiry", "business_accounts"]
    }
  }
}
```

#### 2. Update `config.json`

Add the HELLO operation to your website_content configuration:

```json
"website_content": {
  "name": "website_content",
  "description": "Access website content including news, about page, contact info, and more",
  "category": "content",
  "operations": ["GET", "LIST", "SEARCH", "HELLO"],
  "data_file": "website_content.json"
}
```

#### 3. Create Hello Plugin Handler

Implement the HELLO operation in your website_content plugin:

```rust
// In your plugin implementation
fn execute(&self, operation: &str, params: &Value) -> PluginResult {
    match operation {
        "HELLO" => {
            // Extract optional parameters
            let context = params.get("context").and_then(|c| c.as_str()).unwrap_or("");
            let detail_level = params.get("detail_level").and_then(|d| d.as_str()).unwrap_or("standard");
            
            // Get hello configuration
            let hello_config = self.load_hello_config()?;
            
            // Generate appropriate response based on context and detail level
            self.generate_hello_response(hello_config, context, detail_level)
        },
        // Other operations...
        _ => Err(format!("Unsupported operation: {}", operation).into())
    }
}

fn load_hello_config(&self) -> Result<Value, Box<dyn Error + Send + Sync>> {
    let config_path = Path::new(&self.data_path).join("hello_config.json");
    if config_path.exists() {
        let data = fs::read_to_string(config_path)?;
        Ok(serde_json::from_str(&data)?)
    } else {
        // Fall back to generating from basic website data if no config exists
        let website_data = self.load_data()?;
        Ok(self.generate_default_hello_config(website_data))
    }
}

fn generate_hello_response(
    &self, 
    config: Value, 
    context: &str, 
    detail_level: &str
) -> PluginResult {
    // Default introduction
    let mut intro_text = config.get("default")
        .and_then(|d| d.get("introduction"))
        .and_then(|i| i.as_str())
        .unwrap_or("Welcome to our website.")
        .to_string();
    
    let mut metadata = config.get("default")
        .and_then(|d| d.get("metadata").cloned())
        .unwrap_or_else(|| json!({}));
    
    // Apply context-specific customization if available
    if !context.is_empty() {
        if let Some(contexts) = config.get("contexts") {
            // Look for exact context match
            if let Some(context_config) = contexts.get(context) {
                // Override with context-specific introduction if available
                if let Some(ctx_intro) = context_config.get("introduction").and_then(|i| i.as_str()) {
                    intro_text = ctx_intro.to_string();
                }
                
                // Add context-specific capabilities highlighting
                if let Some(capabilities) = context_config.get("highlight_capabilities") {
                    metadata["highlight_capabilities"] = capabilities.clone();
                }
            }
        }
    }
    
    // Adjust detail level
    let result = match detail_level {
        "basic" => {
            // Simplify metadata for basic requests
            let basic_metadata = json!({
                "provider": metadata.get("provider").cloned().unwrap_or_else(|| json!({}))
            });
            
            json!({
                "content": [{"type": "text", "text": intro_text}],
                "metadata": basic_metadata
            })
        },
        "detailed" => {
            // For detailed requests, include everything
            json!({
                "content": [{"type": "text", "text": intro_text}],
                "metadata": metadata
            })
        },
        _ => {
            // Standard level is the default
            json!({
                "content": [{"type": "text", "text": intro_text}],
                "metadata": {
                    "provider": metadata.get("provider").cloned().unwrap_or_else(|| json!({})),
                    "capabilities": metadata.get("capabilities").cloned().unwrap_or_else(|| json!([])),
                    "topics": metadata.get("primary_focus").cloned().unwrap_or_else(|| json!([]))
                }
            })
        }
    };
    
    Ok(result)
}
```

### Client-Side Implementation

AI agents and MCPI clients should automatically call the HELLO operation when:
- First discovering a new MCPI-enabled website
- When specific context is available to customize the introduction
- Before diving into other operations to understand capabilities

Example client implementation:

```rust
async fn get_website_introduction(
    websocket: &mut WebSocketStream,
    context: Option<&str>
) -> Result<Value, Error> {
    // Create HELLO request
    let request = MCPRequest {
        jsonrpc: "2.0".to_string(),
        id: json!(generate_id()),
        method: "tools/call".to_string(),
        params: Some(json!({
            "name": "website_content",
            "arguments": {
                "operation": "HELLO",
                "context": context.unwrap_or(""),
                "detail_level": "standard"
            }
        })),
    };
    
    // Send request
    websocket.send(Message::Text(serde_json::to_string(&request)?)).await?;
    
    // Receive response
    if let Some(Ok(Message::Text(response))) = websocket.next().await {
        let parsed: MCPResponse = serde_json::from_str(&response)?;
        if let Some(result) = parsed.result {
            return Ok(result);
        } else if let Some(error) = parsed.error {
            return Err(format!("Error response: {}", error.message).into());
        }
    }
    
    Err("No response received".into())
}
```

## Optimization Considerations

When implementing the Hello Protocol, consider these optimization strategies:

### 1. Context Customization

Create context-specific introductions for common user intents:
- Shopping context
- Research context
- Support context
- Business/enterprise context

### 2. Metadata Optimization

Structure your metadata to include:
- Key capabilities most relevant to users
- Primary keywords and topics for AI discovery
- Unique selling points and differentiators
- Relationship links to complementary services

### 3. Performance Considerations

- Keep introduction text concise but informative (aim for 2-4 sentences)
- Prioritize structured metadata over lengthy text descriptions
- Cache frequently used introduction combinations
- Consider implementing fallback options for when hello_config.json is not available

## Example Implementations

### E-commerce Site

```json
{
  "default": {
    "introduction": "Hello! I'm the AI assistant for TechGadgets, an online electronics retailer specializing in smartphones, laptops, and smart home devices. We offer free 2-day shipping on orders over $50 and a 30-day return policy.",
    "metadata": {
      "primary_focus": ["electronics", "gadgets", "tech accessories"],
      "key_offerings": ["smartphones", "laptops", "smart home devices"],
      "unique_selling_points": ["30-day return policy", "free 2-day shipping"]
    }
  },
  "contexts": {
    "shopping": {
      "introduction": "Welcome to TechGadgets! I can help you find the perfect electronic device, check prices, or explore our current promotions."
    },
    "support": {
      "introduction": "Welcome to TechGadgets support! I can help with order status, returns, or technical assistance for products you've purchased."
    }
  }
}
```

### B2B SaaS Platform

```json
{
  "default": {
    "introduction": "Welcome to CloudStack Solutions, a B2B software platform for enterprise resource planning. Our platform helps medium to large businesses streamline operations, manage inventory, and analyze financial data.",
    "metadata": {
      "primary_focus": ["enterprise software", "ERP", "business operations"],
      "key_offerings": ["inventory management", "financial reporting", "supply chain optimization"],
      "unique_selling_points": ["API-first architecture", "unlimited users", "custom workflows"]
    }
  },
  "contexts": {
    "developer": {
      "introduction": "Welcome to CloudStack Solutions developer resources. I can help you with API documentation, integration guides, and SDK information."
    },
    "enterprise": {
      "introduction": "Welcome to CloudStack Solutions for enterprise. Our platform serves 40% of Fortune 500 companies with customizable ERP solutions scaled for complex organizational needs."
    }
  }
}
```

## Benefits for AI Agents and Websites

### Benefits for AI Agents

- **Efficiency**: Obtain critical information about websites without parsing entire sites
- **Consistency**: Standardized method for learning about website capabilities
- **Accuracy**: Direct information from the source rather than inferred from content
- **Context Awareness**: Ability to request information tailored to specific needs

### Benefits for Websites

- **AI SEO**: Optimize for AI agent discovery with controlled introduction content
- **Reduced Misinterpretation**: Directly communicate your value proposition
- **Context Control**: Provide different introductions based on visitor needs
- **Capability Advertising**: Effectively showcase available tools and operations

## Conclusion

The Hello Protocol addresses a critical gap in AI-web interactions by providing an efficient, standardized way for websites to introduce themselves to AI agents. By implementing this protocol, websites can optimize for AI discovery while helping agents provide more relevant assistance to users with less computational overhead.

As AI agents become more prevalent in how users interact with the web, the Hello Protocol will become an essential component of web optimization strategies, enabling websites to effectively communicate with AI intermediaries.
