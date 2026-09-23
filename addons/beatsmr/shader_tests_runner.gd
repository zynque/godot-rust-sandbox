@tool
extends Node

## Root of the combined shader test scene.
##
## Attach each tester as a direct child and press "Run All Tests" in the
## inspector. Testers must expose a run_tests() method (the Rust testers do).
@export_tool_button("Run All Tests")
var run_all_tests_action = run_all_tests

func run_all_tests() -> void:
	for child in get_children():
		if not child.has_method("run_tests"):
			push_error("shader_tests: child '%s' has no run_tests() method." % child.name)
			return
		child.call("run_tests")
