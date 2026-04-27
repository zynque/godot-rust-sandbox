struct FieldNode {
    // spatial
    position: Vec3,
    orientation: Quat,
    scale: Vec3,

    // appearance / rendering
    density: f32,
    color: Vec3,
    anisotropy: Vec3, // elongated splats

    // behavior
    kind: NodeKind,
    params: ParamBlockId,

    // structure
    parent: Option<NodeId>,
    first_child: Option<NodeId>,
    next_sibling: Option<NodeId>,
}
