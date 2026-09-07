@tool
extends InsertionSortByFloatTester

@export_tool_button("Run Insertion Sort Tests")
var run_insertion_sort_tests_action = _run_insertion_sort_tests

func _run_insertion_sort_tests() -> void:
	run_tests()
