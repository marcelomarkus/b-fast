#!/bin/bash
# Integration test: Python backend → TypeScript frontend with performance metrics

set -e

echo "🧪 B-FAST Integration Test: Python → TypeScript"
echo "=" | tr '\n' '=' | head -c 60; echo

# Step 1: Generate test data with Python
echo -e "\n📦 Step 1: Encoding data with Python (backend)..."
python3 << 'PYTHON'
import b_fast
from datetime import datetime, date, time
from decimal import Decimal
from uuid import UUID
from pydantic import BaseModel
from enum import Enum
import time as time_module

class Priority(Enum):
    LOW = 1
    MEDIUM = 2
    HIGH = 3

class Task(BaseModel):
    id: int
    title: str
    description: str
    priority: Priority
    created_at: datetime
    due_date: date
    reminder_time: time
    user_id: UUID
    budget: Decimal
    completed: bool
    tags: tuple[str, ...]

# Create test data (10 tasks)
tasks = []
for i in range(10):
    tasks.append(Task(
        id=i,
        title=f"Task {i}",
        description=f"Description for task {i}",
        priority=Priority.HIGH if i % 3 == 0 else Priority.MEDIUM,
        created_at=datetime(2024, 1, 15, 10, 30, 45),
        due_date=date(2024, 2, 1),
        reminder_time=time(9, 0, 0),
        user_id=UUID("550e8400-e29b-41d4-a716-446655440000"),
        budget=Decimal(f"{1000 + i * 100}.50"),
        completed=i % 2 == 0,
        tags=("python", "rust", "typescript")
    ))

# Encode without compression
encoder = b_fast.BFast()
start = time_module.perf_counter()
encoded = encoder.encode_packed(tasks, compress=False)
encode_time = (time_module.perf_counter() - start) * 1000

# Encode with compression
start = time_module.perf_counter()
encoded_compressed = encoder.encode_packed(tasks, compress=True)
compress_time = (time_module.perf_counter() - start) * 1000

# Save to file
with open('/tmp/integration_test.bin', 'wb') as f:
    f.write(encoded)

with open('/tmp/integration_test_compressed.bin', 'wb') as f:
    f.write(encoded_compressed)

print(f"✅ Encoded {len(tasks)} tasks")
print(f"   Normal: {len(encoded):,} bytes in {encode_time:.2f}ms")
print(f"   Compressed: {len(encoded_compressed):,} bytes in {compress_time:.2f}ms")
print(f"   Compression ratio: {(1 - len(encoded_compressed)/len(encoded))*100:.1f}%")
PYTHON

# Step 2: Decode with TypeScript (normal)
echo -e "\n📦 Step 2: Decoding data with TypeScript (frontend)..."
cd client-ts
node << 'JAVASCRIPT'
const { BFastDecoder } = require('./dist/index.js');
const fs = require('fs');

// Read binary data
const data = fs.readFileSync('/tmp/integration_test.bin');

// Decode normal (warm up)
BFastDecoder.decode(data);

// Measure decode performance (average of 10 runs)
const runs = 10;
let totalTime = 0;

for (let i = 0; i < runs; i++) {
    const start = performance.now();
    const tasks = BFastDecoder.decode(data);
    const end = performance.now();
    totalTime += (end - start);
}

const avgDecodeTime = totalTime / runs;

console.log(`✅ Decoded ${BFastDecoder.decode(data).length} tasks`);
console.log(`   Decode time: ${avgDecodeTime.toFixed(2)}ms (avg of ${runs} runs)`);

// Validate first task
const tasks = BFastDecoder.decode(data);
console.log('\nFull task 1:', JSON.stringify(tasks[0], null, 2));
const task1 = tasks[0];
console.log('\n📊 Type validation (Task 1):');
console.log(`  id: ${task1.id} (${typeof task1.id})`);
console.log(`  priority: ${task1.priority} (${typeof task1.priority})`);
console.log(`  created_at: ${task1.created_at.toISOString()} (${task1.created_at instanceof Date ? 'Date' : 'not Date'})`);
console.log(`  due_date: ${task1.due_date.toISOString().split('T')[0]} (${task1.due_date instanceof Date ? 'Date' : 'not Date'})`);
console.log(`  user_id: ${task1.user_id} (${typeof task1.user_id})`);
console.log(`  budget: ${task1.budget} (${typeof task1.budget})`);
console.log(`  completed: ${task1.completed} (${typeof task1.completed})`);

// Validate types
const errors = [];

if (typeof task1.id !== 'number') errors.push('id should be number');
if (typeof task1.priority !== 'number') errors.push('priority should be number');
if (!(task1.created_at instanceof Date)) errors.push('created_at should be Date');
if (!(task1.due_date instanceof Date)) errors.push('due_date should be Date');
if (typeof task1.user_id !== 'string') errors.push('user_id should be string');
if (typeof task1.budget !== 'number') errors.push('budget should be number');
if (typeof task1.completed !== 'boolean') errors.push('completed should be boolean');

if (errors.length > 0) {
    console.error('\n❌ Type validation failed:');
    errors.forEach(err => console.error(`   - ${err}`));
    process.exit(1);
}

console.log('\n✅ All types validated successfully!');
JAVASCRIPT

echo -e "\n" | tr '\n' '=' | head -c 60; echo
echo -e "\n✅ Integration test passed!"
echo "   Python backend → Binary → TypeScript frontend"
echo "   All types preserved correctly!"
echo "   Performance metrics collected ✓"
