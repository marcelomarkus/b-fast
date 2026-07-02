import { BFastDecoder } from '../index';
import * as fs from 'fs';

// Read test data
const binaryData = fs.readFileSync('/tmp/bfast_typed_test.bin');
const expected = JSON.parse(fs.readFileSync('/tmp/bfast_typed_expected.json', 'utf-8'));

console.log('🧪 Testing B-FAST Type Preservation\n');
console.log('=' .repeat(60));

try {
    // Decode
    const decoded = BFastDecoder.decode(binaryData);
    const obj = decoded[0];
    
    console.log('\n✅ Decoded successfully\n');
    
    // Validate types
    console.log('Type validation:');
    console.log(`  name: "${obj.name}" (${typeof obj.name}) ${obj.name === expected.name ? '✅' : '❌'}`);
    console.log(`  age: ${obj.age} (${typeof obj.age}) ${obj.age === expected.age ? '✅' : '❌'}`);
    console.log(`  created_at: ${obj.created_at.toISOString()} (${obj.created_at instanceof Date ? 'Date' : typeof obj.created_at}) ${obj.created_at.getTime() === new Date(expected.created_at).getTime() ? '✅' : '❌'}`);
    console.log(`  birth_date: ${obj.birth_date.toISOString().split('T')[0]} (${obj.birth_date instanceof Date ? 'Date' : typeof obj.birth_date}) ${obj.birth_date.toISOString().startsWith(expected.birth_date) ? '✅' : '❌'}`);
    console.log(`  wake_time: "${obj.wake_time}" (${typeof obj.wake_time}) ${obj.wake_time === expected.wake_time ? '✅' : '❌'}`);
    console.log(`  user_id: "${obj.user_id}" (${typeof obj.user_id}) ${obj.user_id === expected.user_id ? '✅' : '❌'}`);
    console.log(`  balance: ${obj.balance} (${typeof obj.balance}) ${obj.balance === parseFloat(expected.balance) ? '✅' : '❌'}`);
    console.log(`  active: ${obj.active} (${typeof obj.active}) ${obj.active === expected.active ? '✅' : '❌'}`);
    
    // Decode large parallel-compressed payload
    console.log('\n🧪 Testing B-FAST Parallel Decompression (Large Payload)\n');
    const largeBinaryData = fs.readFileSync('/tmp/bfast_large_test.bin');
    const decodedLarge = BFastDecoder.decode(largeBinaryData);
    console.log(`  Decoded large array length: ${decodedLarge.length} (expected: 12000) ${decodedLarge.length === 12000 ? '✅' : '❌'}`);
    
    // Check all passed
    const allPassed = 
        obj.name === expected.name &&
        obj.age === expected.age &&
        obj.created_at instanceof Date &&
        obj.birth_date instanceof Date &&
        obj.wake_time === expected.wake_time &&
        obj.user_id === expected.user_id &&
        obj.balance === parseFloat(expected.balance) &&
        obj.active === expected.active &&
        decodedLarge.length === 12000;
    
    console.log('\n' + '='.repeat(60));
    if (allPassed) {
        console.log('\n✅ All type preservation tests passed!');
        process.exit(0);
    } else {
        console.log('\n❌ Some tests failed');
        process.exit(1);
    }
    
} catch (error) {
    console.error('\n❌ Error:', error);
    process.exit(1);
}
