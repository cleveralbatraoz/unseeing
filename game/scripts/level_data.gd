class_name LevelData
extends RefCounted
## Placements that belong to the map, not to the systems placed on it: where
## the hero wakes, where the fan stands and faces, which room the hum reveals,
## where the demo tap strikes. MapBuilder is the single author — main only
## consumes, so the geometry and the numbers derived from it can never drift
## apart in two files.

## Where the hero wakes, and the way they face (yaw, radians).
var spawn_pos: Vector3
var spawn_yaw: float

## The fan's floor position and mounting yaw (it pivots around this heading).
var fan_spawn: Vector3
var fan_yaw: float

## The fan's room as wall-centerline bounds (x_min, z_min, x_max, z_max):
## hum waves reveal nothing beyond it — walls stop air.
var hum_room: Vector4

## Dev-demo tap: a fixed point on a real wall, and that wall's outward normal.
var demo_tap: Vector3
var demo_tap_normal: Vector3
