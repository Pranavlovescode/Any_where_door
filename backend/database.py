"""
Database setup and session management using SQLAlchemy
Supports both SQLite (development) and PostgreSQL (production)
"""
from sqlalchemy import create_engine, text
from sqlalchemy.orm import sessionmaker, declarative_base
from config import Settings

settings = Settings()

_is_sqlite = settings.DATABASE_URL.startswith("sqlite")

# Build engine keyword arguments based on the selected backend
_engine_kwargs: dict = {"echo": False}

if _is_sqlite:
    # SQLite needs this flag when used with FastAPI (threaded server)
    _engine_kwargs["connect_args"] = {"check_same_thread": False}
else:
    # PostgreSQL connection pool settings
    _engine_kwargs.update({
        "pool_size": 10,        # Number of persistent connections
        "max_overflow": 20,     # Extra connections allowed beyond pool_size
        "pool_pre_ping": True,  # Verify connections before handing them out
        "pool_recycle": 300,    # Recycle connections every 5 minutes
    })

# Create database engine
engine = create_engine(settings.DATABASE_URL, **_engine_kwargs)

# Create session factory
SessionLocal = sessionmaker(autocommit=False, autoflush=False, bind=engine)

# Declarative base for models
Base = declarative_base()


def get_db():
    """
    Dependency injection function for FastAPI endpoints
    Provides database session and closes it after use
    """
    db = SessionLocal()
    try:
        yield db
    finally:
        db.close()


def init_db():
    """
    Initialize database tables
    Call this on application startup
    """
    Base.metadata.create_all(bind=engine)

    # Log which database backend is active
    if _is_sqlite:
        print(f"  Database backend: SQLite")
    else:
        # Mask password in URL for logging
        _safe_url = settings.DATABASE_URL
        if "@" in _safe_url:
            _safe_url = _safe_url.split("@")[-1]
            _safe_url = f"postgresql://***@{_safe_url}"
        print(f"  Database backend: PostgreSQL ({_safe_url})")

