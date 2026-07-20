from fastapi import FastAPI, HTTPException
from pydantic import BaseModel
from typing import List, Optional
import time

# --- 1. Initialize App and Database Store ---
app = FastAPI(title="FastAPI CRUD Example")

# In-memory "Database": Key is the ID (int), Value is the model instance.
fake_items_db: dict[int, Item] = {}
next_id = 1

# --- 2. Pydantic Schemas (Models) ---
# Defines the structure of data coming in and going out.

class Item(BaseModel):
    """Schema for an item (Product/Resource)."""
    name: str
    description: Optional[str] = None
    price: float
    owner_email: str


class ItemCreate(Item):
    """Schema used when creating a new item."""
    pass

class ItemUpdate(BaseModel):
    """Schema used when updating an existing item (all fields optional)."""
    name: Optional[str] = None
    description: Optional[str] = None
    price: Optional[float] = None
    owner_email: Optional[str] = None


# --- 3. CRUD Endpoints (API Routes) ---

@app.get("/")
def read_root():
    """Root endpoint for testing connectivity."""
    return {"service": "CRUD API running", "status": "online"}


# CREATE (POST)
@app.post("/items/", response_model=Item, status_code=201)
def create_item(item: ItemCreate):
    """Creates a new item and returns it with an assigned ID."""
    global next_id
    
    # Simulate database write (assign ID and store)
    new_item = Item(name=item.name, description=item.description, price=item.price, owner_email=item.owner_email)
    fake_items_db[next_id] = new_item
    print(f"DEBUG: Created item {next_id} - Name: {new_item.name}") 
    
    # Increment ID for next request
    next_id += 1
    return new_item


# READ ALL (GET)
@app.get("/items/", response_model=List[Item])
def read_items():
    """Retrieves a list of all items currently in the database."""
    return list(fake_items_db.values())


# READ ONE (GET by ID)
@app.get("/items/{item_id}", response_model=Item)
def read_item(item_id: int):
    """Retrieves a specific item by its unique ID."""
    if item_id not in fake_items_db:
        raise HTTPException(status_code=404, detail="Item not found")
    return fake_items_db[item_id]


# UPDATE (PUT)
@app.put("/items/{item_id}", response_model=Item)
def update_item(item_id: int, item_update: ItemUpdate):
    """Updates one or more fields of an existing item."""
    if item_id not in fake_items_db:
        raise HTTPException(status_code=404, detail="Item not found")

    current_item = fake_items_db[item_id]
    
    # Use Pydantic's model merging concept to update the existing object safely
    update_data = item_update.model_dump(exclude_unset=True) 
    
    # Create a new Item model instance with merged data
    updated_item = current_item.model_copy(update={'**': update_data})
    
    fake_items_db[item_id] = updated_item
    return updated_item


# DELETE (DELETE)
@app.delete("/items/{item_id}", status_code=204)
def delete_item(item_id: int):
    """Deletes an item by its ID."""
    if item_id not in fake_items_db:
        raise HTTPException(status_code=404, detail="Item not found")
    
    del fake_items_db[item_id]
    return {"message": f"Item {item_id} deleted successfully"}

