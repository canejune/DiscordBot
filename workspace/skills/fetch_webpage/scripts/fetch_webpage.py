import sys
import newspaper
from newspaper import Article

def fetch_webpage(url):
    try:
        article = Article(url)
        article.download()
        article.parse()
        
        title = article.title
        text = article.text
        
        if not text:
            # Fallback for simple pages
            return f"Title: {title}\n\n[No clean text content extracted. The page might be empty or heavily dynamic.]"
            
        return f"Title: {title}\n\nContent:\n{text[:2000]}..." # Limit text for context
    except Exception as e:
        return f"Error fetching URL: {str(e)}"

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: python fetch_webpage.py <URL>")
        sys.exit(1)
    
    target_url = sys.argv[1]
    print(fetch_webpage(target_url))
