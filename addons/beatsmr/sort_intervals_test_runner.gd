@tool
extends SortIntervalsTester

@export_tool_button("Run Sort Intervals Tests")
var run_sort_intervals_tests_action = _run_sort_intervals_tests

func _run_sort_intervals_tests() -> void:
	run_tests()
