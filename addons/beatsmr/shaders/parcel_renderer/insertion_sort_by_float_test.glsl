#[compute]
#version 450

#define MAX_SORT_ELEMENTS 64u

#include "res://addons/beatsmr/shaders/parcel_renderer/insertion_sort_by_float.glslinc"

layout(local_size_x = 1, local_size_y = 1, local_size_z = 1) in;

layout(set = 0, binding = 0, std430) buffer KeyBuffer {
    // [0 .. n-1] = sortable keys
    float keys[];
};

layout(set = 0, binding = 1, std430) buffer IndexBuffer {
    // [0 .. n-1] = source indices preserved through the sort
    uint indices[];
};

layout(set = 0, binding = 2, std430) buffer SizeBuffer {
    uint size;
};

void main() {
    if (size == 0u || size > MAX_SORT_ELEMENTS) {
        return;
    }

    float local_keys[MAX_SORT_ELEMENTS];
    uint local_indices[MAX_SORT_ELEMENTS];

    for (uint i = 0u; i < size; ++i) {
        local_keys[i] = keys[i];
        local_indices[i] = indices[i];
    }

    insertionSortByFloat(local_keys, local_indices, size);

    for (uint i = 0u; i < size; ++i) {
        keys[i] = local_keys[i];
        indices[i] = local_indices[i];
    }
}
