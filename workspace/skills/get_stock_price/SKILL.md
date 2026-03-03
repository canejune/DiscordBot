---
name: get_stock_price
description: Use this skill to fetch the current stock price of a given ticker using yfinance.
---

# Get Stock Price Skill

This skill allows the agent to retrieve the current market price of a stock by its ticker symbol.

## 🛠 Usage

1.  Identify the stock ticker symbol (e.g., AAPL for Apple, TSLA for Tesla).
2.  Execute the provided Python script `scripts/get_stock_price.py` using `run_shell_command`.
3.  Pass the ticker symbol as a command-line argument.

### Example
```bash
python3 workspace/skills/get_stock_price/scripts/get_stock_price.py AAPL
```

## 📋 Requirements

-   `yfinance` library must be installed.
-   Active internet connection for API access.

```bash
pip install -r workspace/skills/get_stock_price/requirements.txt
```
