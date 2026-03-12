import json
import os
import glob

def get_current_channel_dir():
    # 1. Check state.json for active sessions
    state_file = "workspace/state.json"
    if os.path.exists(state_file):
        try:
            with open(state_file, "r") as f:
                state = json.load(f)
                active_sessions = state.get("active_sessions", {})
                if active_sessions:
                    # Find the most recently modified session file among active sessions
                    latest_session_path = None
                    latest_time = -1
                    for session_id, session_path in active_sessions.items():
                        if os.path.exists(session_path):
                            mtime = os.path.getmtime(session_path)
                            if mtime > latest_time:
                                latest_time = mtime
                                latest_session_path = session_path
                    
                    if latest_session_path:
                        # session_path is like "workspace/channels/ai/sessions/20260311212558.md"
                        channel_dir = os.path.dirname(os.path.dirname(latest_session_path))
                        if os.path.exists(os.path.join(channel_dir, "index.md")):
                            return channel_dir
        except Exception:
            pass

    # 2. Fallback: Find the most recently modified session file in workspace/channels
    # that has an accompanying index.md
    session_files = glob.glob("workspace/channels/*/sessions/*.md")
    if session_files:
        # Sort by mtime descending
        session_files.sort(key=os.path.getmtime, reverse=True)
        for session_path in session_files:
            channel_dir = os.path.dirname(os.path.dirname(session_path))
            if os.path.exists(os.path.join(channel_dir, "index.md")):
                return channel_dir

    return None

def main():
    channel_dir = get_current_channel_dir()
    if not channel_dir:
        print("Error: Could not determine current channel.")
        return

    index_file = os.path.join(channel_dir, "index.md")
    if not os.path.exists(index_file):
        print(f"Error: No index.md found in {channel_dir}")
        return

    try:
        with open(index_file, "r") as f:
            content = f.read()
            if not content.strip():
                print("No files indexed in this channel.")
            else:
                print(content)
    except Exception as e:
        print(f"Error reading index.md: {e}")

if __name__ == "__main__":
    main()
