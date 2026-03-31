"""
Installation verification and basic functionality test
Run this script to verify the server setup is correct
"""
import sys
import subprocess
import time


def print_section(title):
    """Print a formatted section header"""
    print(f"\n{'=' * 60}")
    print(f"  {title}")
    print(f"{'=' * 60}\n")


def check_python_version():
    """Check if Python 3.8+ is available"""
    print("Checking Python version...")
    version = sys.version_info
    if version.major >= 3 and version.minor >= 8:
        print(f"✓ Python {version.major}.{version.minor}.{version.micro} (OK)")
        return True
    else:
        print(f"✗ Python {version.major}.{version.minor} (requires 3.8+)")
        return False


def check_dependencies():
    """Check if all required packages are installed"""
    print("\nChecking dependencies...")
    
    required = [
        'fastapi',
        'uvicorn',
        'sqlalchemy',
        'pydantic',
        'PyJWT',
        'cryptography',
        'requests',
    ]
    
    missing = []
    for package in required:
        try:
            __import__(package)
            print(f"  ✓ {package}")
        except ImportError:
            print(f"  ✗ {package} (missing)")
            missing.append(package)
    
    if missing:
        print(f"\n✗ Missing packages: {', '.join(missing)}")
        print(f"  Run: pip install -r requirements.txt")
        return False
    
    print("\n✓ All dependencies installed")
    return True


def check_files():
    """Check if required files exist"""
    print("\nChecking required files...")
    
    required_files = [
        'main.py',
        'routes_auth.py',
        'routes_files.py',
        'routes_agent.py',
        'routes_sync.py',
        'auth_utils.py',
        'schemas.py',
        'models.py',
        'database.py',
        'config.py',
        'requirements.txt',
    ]
    
    missing = []
    for file in required_files:
        try:
            with open(file, 'r') as f:
                pass
            print(f"  ✓ {file}")
        except FileNotFoundError:
            print(f"  ✗ {file} (missing)")
            missing.append(file)
    
    if missing:
        print(f"\n✗ Missing files: {', '.join(missing)}")
        return False
    
    print("\n✓ All required files present")
    return True


def check_imports():
    """Verify all modules can be imported"""
    print("\nChecking module imports...")
    
    modules = [
        ('config', 'Settings'),
        ('database', 'engine'),
        ('models', 'User'),
        ('schemas', 'LoginRequest'),
        ('auth_utils', 'verify_jwt'),
        ('routes_auth', 'router as auth_router'),
        ('routes_files', 'router as files_router'),
        ('routes_agent', 'router as agent_router'),
        ('routes_sync', 'router as sync_router'),
        ('main', 'app'),
    ]
    
    failed = []
    for module_name, item in modules:
        try:
            module = __import__(module_name)
            print(f"  ✓ {module_name}")
        except Exception as e:
            print(f"  ✗ {module_name}: {str(e)[:50]}")
            failed.append((module_name, str(e)))
    
    if failed:
        print(f"\n✗ Import errors detected:")
        for module_name, error in failed:
            print(f"    {module_name}: {error}")
        return False
    
    print("\n✓ All modules import successfully")
    return True


def check_database():
    """Check database initialization"""
    print("\nChecking database setup...")
    
    try:
        from database import engine, init_db
        from models import Base
        
        # Try to initialize database
        init_db()
        print("  ✓ Database engine created")
        print("  ✓ Tables created/verified")
        return True
    except Exception as e:
        print(f"  ✗ Database error: {str(e)}")
        return False


def check_storage():
    """Check storage directory"""
    print("\nChecking storage directory...")
    
    import os
    storage_dir = "./storage/files"
    
    try:
        os.makedirs(storage_dir, exist_ok=True)
        # Check if we can write
        test_file = os.path.join(storage_dir, ".test")
        with open(test_file, 'w') as f:
            f.write("test")
        os.remove(test_file)
        print(f"  ✓ Storage directory writable: {storage_dir}")
        return True
    except Exception as e:
        print(f"  ✗ Storage error: {str(e)}")
        return False


def test_api_startup():
    """Test if API can start (quick test)"""
    print("\nTesting API startup (this may take a few seconds)...")
    
    try:
        # This just imports and creates the app, doesn't start server
        from main import app
        print("  ✓ FastAPI app created successfully")
        return True
    except Exception as e:
        print(f"  ✗ API startup error: {str(e)}")
        return False


def main():
    """Run all checks"""
    print_section("AnywhereDoor Server - Installation Verification")
    
    checks = [
        ("Python Version", check_python_version),
        ("Dependencies", check_dependencies),
        ("Required Files", check_files),
        ("Module Imports", check_imports),
        ("Database Setup", check_database),
        ("Storage Directory", check_storage),
        ("API Startup", test_api_startup),
    ]
    
    results = []
    for check_name, check_func in checks:
        try:
            result = check_func()
            results.append((check_name, result))
        except Exception as e:
            print(f"\n✗ Unexpected error in {check_name}:")
            print(f"  {str(e)}")
            results.append((check_name, False))
    
    # Summary
    print_section("Summary")
    
    passed = sum(1 for _, result in results if result)
    total = len(results)
    
    for check_name, result in results:
        status = "✓ PASS" if result else "✗ FAIL"
        print(f"{status:<8} {check_name}")
    
    print(f"\nTotal: {passed}/{total} checks passed\n")
    
    if passed == total:
        print("✓✓✓ All checks passed! Server is ready. ✓✓✓")
        print("\nYou can now start the server with:")
        print("  Linux/Mac:  ./run_server.sh")
        print("  Windows:    run_server.bat")
        print("\nOr manually:")
        print("  uvicorn main:app --reload")
        return 0
    else:
        print("✗✗✗ Some checks failed. Please fix the issues above. ✗✗✗")
        print("\nCommon solutions:")
        print("  1. Install dependencies: pip install -r requirements.txt")
        print("  2. Check Python version: python3 --version")
        print("  3. Ensure all files are present in this directory")
        print("  4. Check file permissions")
        return 1


if __name__ == "__main__":
    sys.exit(main())
