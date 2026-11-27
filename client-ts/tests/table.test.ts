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

test("BFastTable - typed array views (Float32, Uint32, Uint8) for WebGL/GPU", () => {
    const records = [
        { id: 1, x: 1.25, flags: 0x01, idx: 100 },
        { id: 2, x: 2.50, flags: 0x02, idx: 200 },
        { id: 3, x: 3.75, flags: 0x04, idx: 300 },
    ];

    const encoded = BFastEncoder.encode(records, { compress: false });
    const table = BFastTable.from(encoded);

    // Float32Array for GPU shaders
    const f32 = table.getFloat32Array("x");
    assert.ok(f32 instanceof Float32Array);
    assert.equal(f32.length, 3);
    assert.equal(f32[0], 1.25);
    assert.equal(f32[1], 2.50);
    assert.equal(f32[2], 3.75);

    // Uint32Array for indices/buffers
    const u32 = table.getUint32Array("idx");
    assert.ok(u32 instanceof Uint32Array);
    assert.equal(u32.length, 3);
    assert.equal(u32[0], 100);
    assert.equal(u32[2], 300);

    // Uint8Array for byte flags/colors
    const u8 = table.getUint8Array("flags");
    assert.ok(u8 instanceof Uint8Array);
    assert.equal(u8.length, 3);
    assert.equal(u8[0], 1);
    assert.equal(u8[1], 2);
    assert.equal(u8[2], 4);

    table.free();
});

test("BFastTable - null/validity bitmap checking and isNull", () => {
    const dict = {
        ids: [1, 2, 3, 4],
        scores: [10.5, null, 25.0, NaN],
        names: ["alice", null, "charlie", undefined],
    };

    const encoded = BFastEncoder.encode(dict, { compress: false });
    const table = BFastTable.from(encoded);

    assert.equal(table.rowCount, 4);

    // Validity bitmap for scores
    const scoreValidity = table.getValidityBitmap("scores");
    assert.ok(scoreValidity instanceof Uint8Array);
    assert.equal(scoreValidity.length, 4);
    assert.equal(scoreValidity[0], 1); // 10.5
    assert.equal(scoreValidity[1], 0); // null
    assert.equal(scoreValidity[2], 1); // 25.0
    assert.equal(scoreValidity[3], 0); // NaN

    assert.equal(table.isNull("scores", 0), false);
    assert.equal(table.isNull("scores", 1), true);
    assert.equal(table.isNull("scores", 2), false);
    assert.equal(table.isNull("scores", 3), true);

    // Validity bitmap for names
    const nameValidity = table.getValidityBitmap("names");
    assert.equal(nameValidity[0], 1); // "alice"
    assert.equal(nameValidity[1], 0); // null
    assert.equal(nameValidity[2], 1); // "charlie"
    assert.equal(nameValidity[3], 0); // undefined

    assert.equal(table.isNull("names", 0), false);
    assert.equal(table.isNull("names", 1), true);
    assert.equal(table.isNull("names", 2), false);
    assert.equal(table.isNull("names", 3), true);

    // Out of bounds check
    assert.throws(() => table.isNull("scores", 10), /out of bounds/);

    table.free();
});

test("BFastTable - table concatenation and append for streaming chunks", () => {
    const chunk1Records = [
        { id: 1, val: 10.0, label: "a", active: true },
        { id: 2, val: 20.0, label: "b", active: false },
    ];
    const chunk2Records = [
        { id: 3, val: 30.0, label: "c", active: true },
        { id: 4, val: 40.0, label: "d", active: false },
    ];

    const t1 = BFastTable.from(BFastEncoder.encode(chunk1Records, { compress: false }));
    const t2 = BFastTable.from(BFastEncoder.encode(chunk2Records, { compress: false }));

    // Test append
    const appended = t1.append(t2);
    assert.equal(appended.rowCount, 4);
    assert.deepEqual(appended.getFloat64Array("val"), new Float64Array([10.0, 20.0, 30.0, 40.0]));
    assert.deepEqual(appended.getStringArray("label"), ["a", "b", "c", "d"]);
    assert.deepEqual(appended.getBooleanArray("active"), new Uint8Array([1, 0, 1, 0]));

    // Test static concat with multiple chunks
    const chunk3Records = [
        { id: 5, val: 50.0, label: "e", active: true },
    ];
    const t3 = BFastTable.from(BFastEncoder.encode(chunk3Records, { compress: false }));

    const concatenated = BFastTable.concat([t1, t2, t3]);
    assert.equal(concatenated.rowCount, 5);
    assert.deepEqual(concatenated.getFloat64Array("val"), new Float64Array([10.0, 20.0, 30.0, 40.0, 50.0]));
    assert.deepEqual(concatenated.getStringArray("label"), ["a", "b", "c", "d", "e"]);

    // Edge cases for concat
    assert.equal(BFastTable.concat([]).rowCount, 0);
    assert.equal(BFastTable.concat([t1]).rowCount, 2);

    t1.free();
    t2.free();
    t3.free();
    appended.free();
    concatenated.free();
});

test("BFastTable - visualization adapters: toECharts, toDeckGL, toPlotly", () => {
    const records = [
        { x: 1.5, y: 10.0, count: 100, category: "alpha", active: true },
        { x: 2.5, y: 20.0, count: 200, category: "beta", active: false },
        { x: 3.5, y: 30.0, count: 300, category: "gamma", active: true },
    ];

    const table = BFastTable.from(BFastEncoder.encode(records, { compress: false }));

    // 1. Apache ECharts
    const echarts = table.toECharts();
    assert.ok(Array.isArray(echarts.dimensions));
    assert.ok(echarts.source["x"] instanceof Float64Array);
    assert.equal(echarts.source["x"].length, 3);
    assert.deepEqual(echarts.source["category"], ["alpha", "beta", "gamma"]);

    // Custom subset of columns for ECharts
    const echartsSubset = table.toECharts(["x", "y"]);
    assert.deepEqual(echartsSubset.dimensions, ["x", "y"]);
    assert.equal(Object.keys(echartsSubset.source).length, 2);

    // 2. Deck.gl layer binary attributes
    const deckgl = table.toDeckGL({
        getPosition: { column: "x", size: 1, type: "float32" },
        getColor: { column: "count", size: 1, type: "uint8" },
        getWeight: { column: "y", size: 1, type: "float64" },
    });
    assert.equal(deckgl.length, 3);
    assert.ok(deckgl.attributes.getPosition.value instanceof Float32Array);
    assert.equal(deckgl.attributes.getPosition.size, 1);
    assert.ok(deckgl.attributes.getColor.value instanceof Uint8Array);
    assert.ok(deckgl.attributes.getWeight.value instanceof Float64Array);

    // 3. Plotly WebGL trace format
    const plotly = table.toPlotly();
    assert.ok(plotly["x"] instanceof Float64Array);
    assert.ok(plotly["count"] instanceof Int32Array);
    assert.deepEqual(plotly["category"], ["alpha", "beta", "gamma"]);

    table.free();
});
