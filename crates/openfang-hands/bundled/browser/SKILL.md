---
name: browser-automation
version: "1.0.0"
description: Chrome DevTools MCP-first browser automation patterns for autonomous web interaction
author: OpenFang
tags: [browser, automation, chrome, devtools, mcp, web, scraping]
tools: [mcp_chrome_devtools_new_page, mcp_chrome_devtools_navigate_page, mcp_chrome_devtools_take_snapshot, mcp_chrome_devtools_click, mcp_chrome_devtools_fill, mcp_chrome_devtools_take_screenshot, browser_navigate, browser_click, browser_type, browser_screenshot, browser_read_page, browser_close]
runtime: prompt_only
---

# Browser Automation Skill

Prefer the Chrome DevTools MCP toolchain when it is available. Use builtin `browser_*` tools as a fallback path only.

## Chrome DevTools MCP Interaction Reference

### Core Flow
| Tool | Use Case |
|------|----------|
| `mcp_chrome_devtools_new_page` | Open a fresh tab |
| `mcp_chrome_devtools_select_page` | Switch to the correct active page |
| `mcp_chrome_devtools_navigate_page` | Go to a URL or reload |
| `mcp_chrome_devtools_take_snapshot` | Read the current structured page state and get `uid`s |
| `mcp_chrome_devtools_click` | Click an element by `uid` |
| `mcp_chrome_devtools_fill` / `fill_form` | Fill inputs by `uid` |
| `mcp_chrome_devtools_press_key` | Submit forms, shortcuts, or keyboard navigation |
| `mcp_chrome_devtools_wait_for` | Wait for target text or content changes |
| `mcp_chrome_devtools_take_screenshot` | Visual verification |

### Debugging & Extraction
| Tool | Use Case |
|------|----------|
| `mcp_chrome_devtools_evaluate_script` | Structured DOM reads or extraction |
| `mcp_chrome_devtools_list_console_messages` | Check client-side errors |
| `mcp_chrome_devtools_list_network_requests` | Inspect XHR/fetch activity |
| `mcp_chrome_devtools_get_network_request` | Read request/response details |

## Builtin Browser Fallback

If the MCP server is unavailable or the task is simpler with the builtin browser runtime, fall back to:
- `browser_navigate`
- `browser_click`
- `browser_type`
- `browser_read_page`
- `browser_screenshot`
- `browser_close`

### Basic Selectors
| Selector | Description | Example |
|----------|-------------|---------|
| `#id` | By ID | `#checkout-btn` |
| `.class` | By class | `.add-to-cart` |
| `tag` | By element | `button`, `input` |
| `[attr=val]` | By attribute | `[data-testid="submit"]` |
| `tag.class` | Combined | `button.primary` |

### Form Selectors
| Selector | Use Case |
|----------|----------|
| `input[type="email"]` | Email fields |
| `input[type="password"]` | Password fields |
| `input[type="search"]` | Search boxes |
| `input[name="q"]` | Google/search query |
| `textarea` | Multi-line text areas |
| `select[name="country"]` | Dropdown menus |
| `input[type="checkbox"]` | Checkboxes |
| `input[type="radio"]` | Radio buttons |
| `button[type="submit"]` | Submit buttons |

### Navigation Selectors
| Selector | Use Case |
|----------|----------|
| `a[href*="cart"]` | Cart links |
| `a[href*="checkout"]` | Checkout links |
| `a[href*="login"]` | Login links |
| `nav a` | Navigation menu links |
| `.breadcrumb a` | Breadcrumb links |
| `[role="navigation"] a` | ARIA nav links |

### E-commerce Selectors
| Selector | Use Case |
|----------|----------|
| `.product-price`, `[data-price]` | Product prices |
| `.add-to-cart`, `#add-to-cart` | Add to cart buttons |
| `.cart-total`, `.order-total` | Cart total |
| `.quantity`, `input[name="quantity"]` | Quantity selectors |
| `.checkout-btn`, `#checkout` | Checkout buttons |

## Common Workflows

### Product Search & Purchase
```
1. mcp_chrome_devtools_new_page → open tab
2. mcp_chrome_devtools_navigate_page → store homepage
3. mcp_chrome_devtools_take_snapshot → capture current `uid`s
4. mcp_chrome_devtools_fill / type_text → search box
5. mcp_chrome_devtools_click or press_key Enter → submit search
6. mcp_chrome_devtools_take_snapshot → scan results
7. mcp_chrome_devtools_click → desired product
8. mcp_chrome_devtools_take_snapshot → verify product details & price
9. mcp_chrome_devtools_click → "Add to Cart"
10. Navigate to cart and verify total
11. STOP → Report to user, wait for approval
12. Only then continue toward checkout
```

### Account Login
```
1. Navigate to login page
2. Take snapshot and identify username/password `uid`s
3. Fill credentials
4. Click submit or press Enter
5. Snapshot again to verify success
```

### Form Submission
```
1. Navigate to form page
2. Take snapshot → understand structure and `uid`s
3. Fill fields with MCP fill tools
4. Click checkboxes/radio buttons as needed
5. Take screenshot before submit
6. Submit and snapshot again
7. Verify confirmation state
```

### Price Comparison
```
1. For each store:
   a. browser_navigate → store URL
   b. browser_type → search query
   c. browser_read_page → extract prices
   d. memory_store → save price data
2. memory_recall → compare all prices
3. Report findings to user
```

## Error Recovery Strategies

| Error | Recovery |
|-------|----------|
| Element not found | Try alternative selector, use visible text, scroll page |
| Page timeout | Retry navigation, check URL |
| Login required | Inform user, ask for credentials |
| CAPTCHA | Cannot solve — inform user |
| Pop-up/modal | Click dismiss/close button first |
| Cookie consent | Click "Accept" or dismiss banner |
| Rate limited | Wait 30s, retry |
| Wrong page | Use browser_read_page to verify, navigate back |

## Security Checklist

- Verify domain before entering credentials
- Never store passwords in memory_store
- Check for HTTPS before submitting sensitive data
- Report suspicious redirects to user
- Never auto-approve financial transactions
- Warn about phishing indicators (misspelled domains, unusual URLs)
