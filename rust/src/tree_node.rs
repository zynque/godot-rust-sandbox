
type NodeId = usize;

struct Node<A> {
    id: NodeId,
    value: A,
    children: Vec<Node<A>>,
}

struct Tree<A> {
    nodes: Vec<Node<A>>,

}

struct PlantSegment {
    geometry: SegmentGeometry,
    content: PlantSegmentContent,
}

struct SegmentGeometry {
  transform: Vec<[f32; 16]>,
  thickness: Vec<f32>,
  vector: Vec<[f32; 3]>,
}

struct PlantSegmentContent {
    capacity: Vec<f32>,
    nutrient_saturation: Vec<f32>,
    energy_saturation: Vec<f32>,
    sugar_saturation: Vec<f32>,
}

type Plant = Tree<PlantSegment>;
