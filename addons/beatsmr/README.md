BeaTS Mr.
Beam-Traced Statistical Matter Renderer

This renderer represents matter as a collection of gaussian ellipsoids with material properties. Each ellipsoid along with its material properties is called a "Parcel". A single parcel might represent a small cluster of cells in a leaf, for example. A larger parcel might represent a puff of cloud. Collections of nearby parcels define an implicit surface resulting from their combined shapes and densities like Metaballs, but can also be transparent like mist. A Parcel's solidity provides a range between the two.

Parcels form a hierarchy. A parent parcel's ellipsoid and properties summarize the shape and properties of its children, allowing it to be used in place of its children at far distances. The parcel hierarchy also serves as a bounding volume hierarchy, allowing the renderer to quickly disregard large chunks of matter that are not relevant to a drawn pixel. Parcels can be re-used in multiple positions/orientations under multiple parents to allow for complex repeated structures, such as a tree with many similar branches.

We borrows ideas from gaussian splats, ray tracing, and ray marching. Unlike gaussian splats, which collect optical information in gaussian ellipsoids with spherical harmonics, these gaussians represent physical matter itself. For each pixel, a conical beam is cast from the camera and tested for intersections against parcels in the hierarchy. Parcels that do not intersect can be disregarded, along with their children. Intersected parcels will be considered. If near, the children will be evaluated. If far, the parcel itself can be rendered.

Shader development requires a different mindset:
We cannot simply pass a bunch of references around between fine-grained functions.
Passing an array means copying the array.
Instead, prefer global data buffers to minimize copies, and pass indices/ranges.
To retain some semblance of encapsulation and ownership, consider maintaining
a global buffer of entities and ranges representing their children.

Style of glsl files:
functions and variables: snake_case
structs and buffer interface blocks: PascalCase
constants: ALL_CAPS
todo: apply uniformly

A tentative outline suggested by AI (to be refined)

shaders/
│
├── math/
│   ├── matrices.glslinc
│   ├── covariance.glslinc
│   ├── quadratic.glslinc
│   └── random.glslinc
│
├── geometry/
│   ├── ray.glslinc
│   ├── ellipsoid.glslinc
│   ├── bounds.glslinc
│   └── intersections.glslinc
│
├── hierarchy/
│   ├── octree.glslinc
│   ├── parcel.glslinc
│   ├── lod.glslinc
│   └── traversal.glslinc
│
├── density/
│   ├── gaussian.glslinc
│   ├── mixtures.glslinc
│   ├── integration.glslinc
│   └── optical_depth.glslinc
│
├── rendering/
│   ├── lighting.glslinc
│   ├── material.glslinc
│   ├── accumulation.glslinc
│   └── resolve.glslinc
│
└── passes/
		├── visibility.comp
		├── integrate.comp
		└── resolve.comp
