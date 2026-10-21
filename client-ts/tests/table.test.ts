import test from "node:test";
import assert from "node:assert/strict";
import { BFastEncoder, BFastTable } from "../index";

test("BFastTable - parses uncompressed records into zero-copy columnar buffers", () => {
    const records = [
        { id: 1, name: "Alpha", price: 10.5, active: true },
        { id: 2, name: "Beta", price: 20.0, active: false },
        { id: 3, name: "Gamma", price: 35.75, active: true },
        { id: 4, name: "Delta", price: 50.25, active: false },
        { id: 5, name: "Epsilon", price: 100.0, active: true },
    ];

    const encoded = BFastEncoder.encode(records, { compress: false });
    const table = BFastTable.from(encoded);

    assert.equal(table.rowCount, 5);
    assert.deepEqual(table.columnNames.sort(), ["active", "id", "name", "price"]);

    assert.equal(table.getColumnType("price"), "float64");
    assert.equal(table.getColumnType("id"), "int64");
    assert.equal(table.getColumnType("active"), "boolean");
    assert.equal(table.getColumnType("name"), "string");

    // Float64Array view
    const prices = table.getFloat64Array("price");
    assert.equal(prices.length, 5);
    assert.equal(prices[0], 10.5);
    assert.equal(prices[1], 20.0);
    assert.equal(prices[4], 100.0);

    // Int32Array and BigInt64Array views
    const ids = table.getInt32Array("id");
    assert.equal(ids.length, 5);
    assert.equal(ids[0], 1);
    assert.equal(ids[4], 5);

    const bigIds = table.getBigInt64Array("id");
    assert.equal(bigIds[2], 3n);

    // BooleanArray view
    const actives = table.getBooleanArray("active");
    assert.equal(actives.length, 5);
    assert.equal(actives[0], 1);
    assert.equal(actives[1], 0);
    assert.equal(actives[4], 1);

    // String on-demand access
    assert.equal(table.getString("name", 0), "Alpha");
    assert.equal(table.getString("name", 1), "Beta");
    assert.equal(table.getString("name", 4), "Epsilon");
    assert.deepEqual(table.getStringArray("name"), ["Alpha", "Beta", "Gamma", "Delta", "Epsilon"]);

    // Materialize to objects
    const materialized = table.toObjects();
    assert.equal(materialized.length, 5);
    assert.equal(materialized[0].name, "Alpha");
    assert.equal(materialized[0].price, 10.5);
    assert.equal(materialized[0].active, true);

    table.free();
});

test("BFastTable - parses compressed records and handles fast aggregations", () => {
    const N = 1000;
    const records = [];
    let expectedSum = 0;
    for (let i = 0; i < N; i++) {
        const price = (i + 1) * 2.5;
        expectedSum += price;
        records.push({
            id: i + 1,
            name: `Product_${i}`,
            price: price,
            active: i % 2 === 0,
        });
    }

    const encoded = BFastEncoder.encode(records, { compress: true }); // compressed
    const table = BFastTable.from(encoded);

    assert.equal(table.rowCount, N);

    // Aggregations
    const sumPrice = table.aggregate("price", "sum") as number;
    assert.ok(Math.abs(sumPrice - expectedSum) < 1e-4);

    const avgPrice = table.aggregate("price", "avg") as number;
    assert.ok(Math.abs(avgPrice - (expectedSum / N)) < 1e-4);

    const minPrice = table.aggregate("price", "min") as number;
    assert.equal(minPrice, 2.5);

    const maxPrice = table.aggregate("price", "max") as number;
    assert.equal(maxPrice, N * 2.5);

    const countPrice = table.aggregate("price", "count") as number;
    assert.equal(countPrice, N);

    // Integer aggregation
    const sumId = table.aggregate("id", "sum");
    const expectedIdSum = BigInt((N * (N + 1)) / 2);
    assert.equal(BigInt(sumId), expectedIdSum);

    // Filtering
    const filtered = table.filter("price", ">", 2400.0);
    // price > 2400 => (i + 1) * 2.5 > 2400 => i + 1 > 960 => i >= 960 => 40 items
    assert.equal(filtered.length, 40);
    assert.equal(filtered[0], 960);
    assert.equal(filtered[39], 999);

    table.free();
});

test("BFastTable - handles empty and edge cases", () => {
    const encoded = BFastEncoder.encode([], { compress: false });
    const table = BFastTable.from(encoded);

    assert.equal(table.rowCount, 0);
    assert.equal(table.columnNames.length, 0);
    table.free();

    // Calling methods after disposal throws
    assert.throws(() => table.getFloat64Array("nonexistent"), /freed \/ disposed/);
});

test("BFastTable - dynamic numeric promotion (int -> float) and cross-type views", () => {
    // Row 0 has integer 10, Row 1 has float 15.5 -> column should promote to float64
    const records = [
        { id: 1, val: 10 },
        { id: 2, val: 15.5 },
        { id: 3, val: 20 },
    ];
    const encoded = BFastEncoder.encode(records, { compress: false });
    const table = BFastTable.from(encoded);

    assert.equal(table.rowCount, 3);
    assert.equal(table.getColumnType("val"), "float64");

    const vals = table.getFloat64Array("val");
    assert.equal(vals[0], 10.0);
    assert.equal(vals[1], 15.5);
    assert.equal(vals[2], 20.0);

    // Cross-type access: ask for float64 array on an int64 column
    const idFloats = table.getFloat64Array("id");
    assert.equal(idFloats[0], 1.0);
    assert.equal(idFloats[2], 3.0);

    // Cross-type access: ask for int32 array on a float64 column
    const valInts = table.getInt32Array("val");
    assert.equal(valInts[0], 10);
    assert.equal(valInts[1], 15);
    assert.equal(valInts[2], 20);

    table.free();
});

test("BFastTable - columnar dictionary fallback", () => {
    // Columnar dictionary representation
    const dict = {
        ids: [100, 200, 300],
        scores: [95.5, 88.0, 72.25],
        labels: ["first", "second", "third"],
    };
    const encoded = BFastEncoder.encode(dict, { compress: false });
    const table = BFastTable.from(encoded);

    assert.equal(table.rowCount, 3);
    assert.ok(table.columnNames.includes("ids"));
    assert.ok(table.columnNames.includes("scores"));
    assert.ok(table.columnNames.includes("labels"));

    table.free();
});
