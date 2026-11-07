# The baseline: no workflow, nothing seeded, no hook added. The Job gets the yardstick
# brief and Armada's own session rules and mod, as any hosted Job does.
#
# **No workflow needs a setting, not an absence.** With `features.workflows` on, Fleet's
# proposer reads the first message and picks a workflow from the repository (the 'bug' one
# for the yardstick), tells the agent its steps and asks the person how to deliver. So
# this variant turns the setting off in the scratch Fleet's settings.json (read live), and
# a variant that wants workflows turns it back on.
#
# A variant file may define these (all optional), and run.sh calls them in this order.
# Each runs with $port, $scratch, $out and $rig set.
#   variant_prepare              before the Job exists: change the clone, the mod copy, the settings
#   variant_start_extra          print extra JSON members for POST /sessions/start
#   variant_after_start <id>     after the Job exists and before its first message
settings_set() { # settings_set <dotted.key> <json value>; settings.json is one flat object of dotted keys
  python3 -I - "$scratch/home/user/Library/Application Support/Armada/settings.json" "$1" "$2" <<'PY'
import json, os, sys
path, key, value = sys.argv[1:]
data = json.load(open(path)) if os.path.exists(path) else {}
data[key] = json.loads(value)
json.dump(data, open(path, "w"), indent=2)
PY
  # Fleet polls the file every second and waits for it to sit still 1.5 s before it reads it.
  sleep 5
}

variant_prepare() { settings_set features.workflows false; }
variant_start_extra() { :; }
variant_after_start() { :; }
