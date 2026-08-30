#[compute]
#version 450

// ---------------------------------------------------------------------------
// Generic insertion sort over float-valued keys.
//
// Inspired by sortBoundaries() in shadertoy_parcel_rendering.glsl, but
// generalised so that it can sort any array of (key, payload) pairs rather
// than only the fixed BoundaryIntervalSet structure.
//
// Layout
// ------
// Binding 0 (storage, read-write):
//   A flat array of floats packed as
//       [ n, key0, key1, ..., key_{n-1} ]
//   where n is a uint32 stored as a float (must be an integer value ≤
//   MAX_ELEMENTS).  The shader sorts keys[0..n) in ascending order in-place
//   and writes them back to the same binding so the host can read the result.
// ---------------------------------------------------------------------------

layout(local_size_x = 1, local_size_y = 1, local_size_z = 1) in;

#define MAX_ELEMENTS 64

layout(set = 0, binding = 0, std430) buffer SortBuffer {
    // [0]        = n (element count, stored as float)
    // [1 .. n]   = keys to sort
    float data[];
};

void insertionSort(uint n) {
    for (uint i = 1u; i < n; i++) {
        float key = data[i + 1u];
        int j = int(i) - 1;
        while (j >= 0 && data[uint(j) + 1u] > key) {
            data[uint(j) + 2u] = data[uint(j) + 1u];
            j--;
        }
        data[uint(j) + 2u] = key;
    }
}

void main() {
    uint n = uint(data[0]);
    if (n == 0u || n > uint(MAX_ELEMENTS)) {
        return;
    }
    insertionSort(n);
}
