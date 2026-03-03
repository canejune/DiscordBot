import sys
import yfinance as yf

def get_stock_price(ticker):
    try:
        stock = yf.Ticker(ticker)
        # Try to get the latest price from fast_info (usually more efficient)
        try:
            price = stock.fast_info['lastPrice']
        except (KeyError, AttributeError):
            # Fallback: get the last close price from history
            history = stock.history(period="1d")
            if history.empty:
                return f"Error: No data found for ticker '{ticker}'."
            price = history['Close'].iloc[-1]
        
        return f"The current price of {ticker.upper()} is ${price:.2f}"
    except Exception as e:
        return f"Error: {str(e)}"

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: python get_stock_price.py <TICKER>")
        sys.exit(1)
    
    ticker_symbol = sys.argv[1]
    print(get_stock_price(ticker_symbol))
