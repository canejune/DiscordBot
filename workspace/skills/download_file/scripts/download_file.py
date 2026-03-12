import sys

def main():
    if len(sys.argv) < 2:
        print("Usage: python3 download_file.py [filename]")
        return
    
    filename = sys.argv[1]
    print(f"[[download:{filename}]]")

if __name__ == "__main__":
    main()
