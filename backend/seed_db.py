#!/usr/bin/env python3
"""
Database seeding script - Add test users and initial data
Run this once to initialize the database with test data for development
"""

import uuid
from datetime import datetime
from database import SessionLocal, init_db
from models import User, Device
from auth_utils import hash_password

def seed_database():
    """Initialize database with test data"""
    
    # Initialize tables
    print("Initializing database tables...")
    init_db()
    print("✓ Tables created/verified")
    
    # Create session
    db = SessionLocal()
    
    try:
        # Check if test user already exists
        existing_user = db.query(User).filter(User.username == "testuser").first()
        if existing_user:
            print("✓ Test user already exists")
            print(f"  Username: testuser")
            print(f"  User ID: {existing_user.user_id}")
            
            # Show existing devices
            devices = db.query(Device).filter(Device.user_id == existing_user.user_id).all()
            if devices:
                print(f"  Devices: {len(devices)}")
                for device in devices:
                    print(f"    - {device.device_id}")
            return
        
        # Create test user
        test_user_id = str(uuid.uuid4())
        test_user = User(
            user_id=test_user_id,
            username="testuser",
            password_hash=hash_password("testpass123")
        )
        
        db.add(test_user)
        db.commit()
        
        print("✓ Created test user:")
        print(f"  Username: testuser")
        print(f"  Password: testpass123")
        print(f"  User ID: {test_user_id}")
        
        # Create another test user for device testing
        admin_user_id = str(uuid.uuid4())
        admin_user = User(
            user_id=admin_user_id,
            username="admin",
            password_hash=hash_password("admin123")
        )
        
        db.add(admin_user)
        db.commit()
        
        print("✓ Created admin user:")
        print(f"  Username: admin")
        print(f"  Password: admin123")
        print(f"  User ID: {admin_user_id}")
        
        print("\n✓ Database seeding complete!")
        print("\nYou can now test the API:")
        print("  1. Login: POST /auth/login with username=testuser, password=testpass123")
        print("  2. Register Device: POST /auth/register-device with JWT from step 1")
        print("  3. Run agent: Set ANYWHERE_DOOR_USER_JWT and run cargo run")
        
    except Exception as e:
        print(f"❌ Error seeding database: {e}")
        db.rollback()
        raise
    finally:
        db.close()

if __name__ == "__main__":
    seed_database()
