import json
import sys
import os

TASKS_FILE = "workspace/tasks.json"

def load_tasks():
    if not os.path.exists(TASKS_FILE):
        return {"tasks": []}
    try:
        with open(TASKS_FILE, "r") as f:
            return json.load(f)
    except Exception as e:
        print(f"Error loading tasks: {e}")
        return {"tasks": []}

def save_tasks(tasks):
    try:
        with open(TASKS_FILE, "w") as f:
            json.dump(tasks, f, indent=2)
    except Exception as e:
        print(f"Error saving tasks: {e}")

def list_tasks():
    tasks = load_tasks()
    if not tasks["tasks"]:
        print("No triggers registered.")
        return
    
    print("Available Triggers:")
    for task in tasks["tasks"]:
        print(f"- ID: {task['id']}")
        print(f"  Prompt: {task['prompt']}")
        print("-" * 20)

def add_task(task_id, prompt):
    tasks = load_tasks()
    # Check if ID already exists
    if any(task["id"] == task_id for task in tasks["tasks"]):
        print(f"Error: Trigger ID '{task_id}' already exists.")
        return
    
    tasks["tasks"].append({"id": task_id, "prompt": prompt})
    save_tasks(tasks)
    print(f"Trigger '{task_id}' added successfully.")

def remove_task(task_id):
    tasks = load_tasks()
    original_count = len(tasks["tasks"])
    tasks["tasks"] = [task for task in tasks["tasks"] if task["id"] != task_id]
    
    if len(tasks["tasks"]) == original_count:
        print(f"Error: Trigger ID '{task_id}' not found.")
    else:
        save_tasks(tasks)
        print(f"Trigger '{task_id}' removed successfully.")

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: trigger_manager.py [list|add|remove] [id] [prompt]")
        sys.exit(1)
    
    command = sys.argv[1].lower()
    
    if command == "list":
        list_tasks()
    elif command == "add":
        if len(sys.argv) < 4:
            print("Usage: trigger_manager.py add <id> <prompt>")
            sys.exit(1)
        add_task(sys.argv[2], sys.argv[3])
    elif command == "remove":
        if len(sys.argv) < 3:
            print("Usage: trigger_manager.py remove <id>")
            sys.exit(1)
        remove_task(sys.argv[2])
    else:
        print(f"Unknown command: {command}")
        sys.exit(1)
