@tool
extends Node

## Root of the combined shader test scene.
##
## Attach each tester as a direct child and press "Run All Tests" in the
## inspector. Testers must expose a run_tests() method returning a Vector2i of
## (passed, total). Once every tester has run the scene prints whether all
## tests passed and the grand total across all of them.
@export_tool_button("Run All Tests")
var run_all_tests_action = run_all_tests

func run_all_tests() -> void:
	var passed := 0
	var total := 0
	for child in get_children():
		if not child.has_method("run_tests"):
			push_error("shader_tests: child '%s' has no run_tests() method." % child.name)
			return
		var report: Variant = child.call("run_tests")
		if not report is Vector2i:
			push_error("shader_tests: child '%s' returned no (passed, total) report." % child.name)
			return
		passed += report.x
		total += report.y

	if passed == total:
		print("shader_tests: ALL TESTS PASSED (%d/%d)." % [passed, total])
	else:
		print("shader_tests: TESTS FAILED (%d/%d passed)." % [passed, total])
