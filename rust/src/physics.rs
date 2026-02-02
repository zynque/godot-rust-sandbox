use godot::prelude::*;
use godot::builtin::{
  Rid
};
use godot::classes::{
    PhysicsServer2D,
    PhysicsDirectSpaceState2D,
    PhysicsShapeQueryParameters2D,
    Shape2D,
    Engine,
    SceneTree,
};

pub struct GodotPhysics{
  space: Rid,
  state_space: Gd<PhysicsDirectSpaceState2D>,
  used_rids: Vec<Rid>,
}

pub trait GodotPhysicsSpace {
  fn add_area_polygon(&self, polygon: &Vec<Vector2>, position: Vector2) -> Rid;
  fn polygon_collides(&mut self, polygon: &Vec<Vector2>, position: Vector2) -> bool;
  fn cast_motion(&mut self, polygon: &Vec<Vector2>, current_pos: Vector2, movement: Vector2) -> MoveResult;
}

#[derive(Debug, Clone, Copy)]
pub struct MoveResult {
  pub motion: Vector2,
  pub remainder: Vector2,
  pub collided: bool,
}

// fn world_space_physics() -> Result<GodotPhysics> {
//   let mut server = PhysicsServer2D::singleton();
  
//   let world = Engine::singleton()
//     .get_main_loop()?
//     .try_cast::<SceneTree>()?
//     .get_root()?
//     .get_world_2d()?;
// }

pub fn new_physics_space() -> Option<GodotPhysics> {
  let mut server = PhysicsServer2D::singleton();
  let space = server.space_create();
  Some(GodotPhysics {
    space,
    state_space: server.space_get_direct_state(space)?,
    used_rids: Vec::new(),
  })
}

impl GodotPhysicsSpace for GodotPhysics {
  fn add_area_polygon(&self, polygon: &Vec<Vector2>, position: Vector2) -> Rid {
    let mut server = PhysicsServer2D::singleton();
    let shape = server.convex_polygon_shape_create();
    server.shape_set_data(shape, &Variant::from(PackedVector2Array::from(polygon.as_slice())));
    let area = server.area_create();
    server.area_set_space(area, self.space);
    server.area_add_shape(area, shape);
    let transform = Transform2D::IDENTITY.translated(position);
    server.area_set_transform(area, transform);
    // TODO: Collision Mask
    area
  }

  fn polygon_collides(&mut self, polygon: &Vec<Vector2>, position: Vector2) -> bool {
    let mut server = PhysicsServer2D::singleton();
    let mut physics_query = PhysicsShapeQueryParameters2D::new_gd();
    
    let shape = server.convex_polygon_shape_create();
    server.shape_set_data(shape, &Variant::from(PackedVector2Array::from(polygon.as_slice())));

    let transform = Transform2D::IDENTITY.translated(position);

    physics_query.set_shape_rid(shape);
    physics_query.set_transform(transform);
    physics_query.set_collide_with_areas(true);
    // TODO: Collision Mask

    let result = self.state_space.intersect_shape(&physics_query);
    !result.is_empty()
  }

  fn cast_motion(&mut self, polygon: &Vec<Vector2>, current_pos: Vector2, movement: Vector2) -> MoveResult {
    let mut server = PhysicsServer2D::singleton();
    let mut physics_query = PhysicsShapeQueryParameters2D::new_gd();
    
    let shape = server.convex_polygon_shape_create();
    server.shape_set_data(shape, &Variant::from(PackedVector2Array::from(polygon.as_slice())));

    let transform = Transform2D::IDENTITY.translated(current_pos);

    physics_query.set_shape_rid(shape);
    physics_query.set_transform(transform);
    physics_query.set_motion(movement);
    physics_query.set_collide_with_areas(true);
    // TODO: Collision Mask

    let result = self.state_space.cast_motion(&physics_query);
    
    // cast_motion returns an array with [safe_fraction, unsafe_fraction]
    // safe_fraction tells us how far we can move before collision (0.0 to 1.0)
    let safe_fraction = result.get(0).unwrap_or(1.0);
    
    let motion = movement * safe_fraction;
    let remainder = movement * (1.0 - safe_fraction);
    let collided = safe_fraction < 1.0;
    
    MoveResult {
      motion,
      remainder,
      collided,
    }
  }
}
