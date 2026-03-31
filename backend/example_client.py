"""
Example client script to demonstrate using the AnywhereDoor Server API
Shows how to authenticate and upload files like the Rust agent would
"""
import requests
import json
import base64
import hashlib
import hmac
from datetime import datetime
from typing import Dict, Any


class AnywhereDoorClient:
    """
    Client for interacting with AnywhereDoor Server
    """
    
    def __init__(self, base_url: str = "http://localhost:8000"):
        self.base_url = base_url
        self.jwt = None
        self.device_id = None
        self.device_secret = None
        self.session = requests.Session()
    
    
    def create_user(self, username: str, password: str) -> bool:
        """
        Create a new user account
        """
        url = f"{self.base_url}/auth/create-user"
        params = {"username": username, "password": password}
        
        try:
            response = self.session.post(url, params=params)
            if response.status_code == 200:
                data = response.json()
                print(f"✓ User created: {data['user_id']}")
                return True
            elif response.status_code == 409:
                print("✓ User already exists")
                return True
            else:
                print(f"✗ Failed to create user: {response.json()}")
                return False
        except Exception as e:
            print(f"✗ Error: {str(e)}")
            return False
    
    
    def login(self, username: str, password: str) -> bool:
        """
        Login user and get JWT token
        """
        url = f"{self.base_url}/auth/login"
        payload = {"username": username, "password": password}
        
        try:
            response = self.session.post(url, json=payload)
            if response.status_code == 200:
                data = response.json()
                self.jwt = data["jwt"]
                print(f"✓ Login successful")
                print(f"  JWT: {self.jwt[:50]}...")
                return True
            else:
                print(f"✗ Login failed: {response.json()}")
                return False
        except Exception as e:
            print(f"✗ Error: {str(e)}")
            return False
    
    
    def register_device(self) -> bool:
        """
        Register device and get device credentials
        """
        if not self.jwt:
            print("✗ Must login first")
            return False
        
        url = f"{self.base_url}/auth/register-device"
        payload = {"jwt": self.jwt}
        
        try:
            response = self.session.post(url, json=payload)
            if response.status_code == 200:
                data = response.json()
                self.device_id = data["device_id"]
                self.device_secret = data["device_secret"]
                print(f"✓ Device registered: {self.device_id}")
                print(f"  Secret: {self.device_secret[:32]}...")
                return True
            else:
                print(f"✗ Failed to register device: {response.json()}")
                return False
        except Exception as e:
            print(f"✗ Error: {str(e)}")
            return False
    
    
    def upload_file(self, file_path: str, original_path: str = None) -> bool:
        """
        Upload a file to the server
        """
        if not self.jwt:
            print("✗ Must login first")
            return False
        
        original_path = original_path or file_path
        
        try:
            # Read file
            with open(file_path, "rb") as f:
                file_content = f.read()
            
            # Calculate hash
            file_hash = hashlib.sha256(file_content).hexdigest()
            
            # Encode file content
            encoded_content = base64.b64encode(file_content).decode('utf-8')
            
            # Prepare payload
            metadata = {
                "file_path": original_path,
                "file_name": file_path.split("/")[-1],
                "file_size": len(file_content),
                "modified_at": int(datetime.utcnow().timestamp()),
                "created_at": int(datetime.utcnow().timestamp()),
                "file_hash": file_hash,
                "mime_type": "application/octet-stream",
                "is_directory": False
            }
            
            payload = {
                "metadata": metadata,
                "file_content": encoded_content
            }
            
            # Upload
            url = f"{self.base_url}/api/files/upload"
            headers = {"Authorization": f"Bearer {self.jwt}"}
            params = {"jwt": self.jwt}
            
            response = self.session.post(url, json=payload, headers=headers, params=params)
            
            if response.status_code == 200:
                data = response.json()
                print(f"✓ File uploaded: {data['file_id']}")
                print(f"  Size: {data['size_bytes']} bytes")
                print(f"  Hash verified: {data['hash_verified']}")
                return True
            else:
                print(f"✗ Upload failed: {response.json()}")
                return False
        
        except FileNotFoundError:
            print(f"✗ File not found: {file_path}")
            return False
        except Exception as e:
            print(f"✗ Error: {str(e)}")
            return False
    
    
    def register_agent(self, agent_id: str, hostname: str, os_name: str) -> bool:
        """
        Register an agent
        """
        if not self.jwt:
            print("✗ Must login first")
            return False
        
        url = f"{self.base_url}/api/agent/register"
        payload = {
            "agent_id": agent_id,
            "agent_version": "1.0.0",
            "os": os_name,
            "hostname": hostname,
            "sync_root": "/home/user/Documents",
            "last_sync": int(datetime.utcnow().timestamp()),
            "status": "active"
        }
        
        params = {"jwt": self.jwt}
        
        try:
            response = self.session.post(url, json=payload, params=params)
            if response.status_code == 200:
                data = response.json()
                print(f"✓ Agent registered: {data['agent_id']}")
                return True
            else:
                print(f"✗ Failed to register agent: {response.json()}")
                return False
        except Exception as e:
            print(f"✗ Error: {str(e)}")
            return False
    
    
    def list_files(self, limit: int = 10) -> bool:
        """
        List files for the user
        """
        if not self.jwt:
            print("✗ Must login first")
            return False
        
        url = f"{self.base_url}/api/files/list"
        params = {"jwt": self.jwt, "limit": limit, "skip": 0}
        
        try:
            response = self.session.get(url, params=params)
            if response.status_code == 200:
                data = response.json()
                print(f"✓ Files listed: {data['total']} total")
                for file_info in data['files'][:limit]:
                    print(f"  - {file_info['file_name']} ({file_info['file_size']} bytes)")
                return True
            else:
                print(f"✗ Failed to list files: {response.json()}")
                return False
        except Exception as e:
            print(f"✗ Error: {str(e)}")
            return False
    
    
    def get_agent_status(self, agent_id: str) -> bool:
        """
        Get status of an agent
        """
        if not self.jwt:
            print("✗ Must login first")
            return False
        
        url = f"{self.base_url}/api/agent/{agent_id}/status"
        params = {"jwt": self.jwt}
        
        try:
            response = self.session.get(url, params=params)
            if response.status_code == 200:
                data = response.json()
                print(f"✓ Agent status: {data['status']}")
                print(f"  Files synced: {data['files_synced']}")
                print(f"  Total size: {data['total_size']} bytes")
                return True
            else:
                print(f"✗ Failed to get agent status: {response.json()}")
                return False
        except Exception as e:
            print(f"✗ Error: {str(e)}")
            return False


def main():
    """
    Example usage of the AnywhereDoor client
    """
    print("\n" + "="*60)
    print("AnywhereDoor Server - Example Client")
    print("="*60 + "\n")
    
    # Create client
    client = AnywhereDoorClient("http://localhost:8000")
    
    # Create user
    print("[1] Creating user...")
    client.create_user("testuser", "testpass123")
    print()
    
    # Login
    print("[2] Logging in...")
    if not client.login("testuser", "testpass123"):
        return
    print()
    
    # Register device
    print("[3] Registering device...")
    if not client.register_device():
        return
    print()
    
    # Register agent
    print("[4] Registering agent...")
    if not client.register_agent("agent-001", "my-laptop", "Linux"):
        return
    print()
    
    # Create test file
    print("[5] Creating test file...")
    test_file = "/tmp/test_upload.txt"
    with open(test_file, "w") as f:
        f.write("This is a test file for AnywhereDoor\n" * 100)
    print(f"✓ Test file created: {test_file}")
    print()
    
    # Upload file
    print("[6] Uploading file...")
    if not client.upload_file(test_file, "/home/user/Documents/test.txt"):
        return
    print()
    
    # List files
    print("[7] Listing files...")
    client.list_files()
    print()
    
    # Get agent status
    print("[8] Getting agent status...")
    client.get_agent_status("agent-001")
    print()
    
    print("="*60)
    print("Example completed successfully!")
    print("="*60 + "\n")


if __name__ == "__main__":
    main()
