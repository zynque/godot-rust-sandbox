@tool
extends IntersectParcelsTester

@export_tool_button("Run Intersect Parcels Tests")
var run_intersect_parcels_tests_action = _run_intersect_parcels_tests

func _run_intersect_parcels_tests() -> void:
	run_tests()
