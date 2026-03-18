# ConvergioMesh Deprecation Notice

As of Plan 664, ConvergioMesh has been merged into ConvergioPlatform/daemon/.

## What happened
- All mesh networking modules merged into `daemon/src/mesh/`
- CLI modules moved to `daemon/src/cli/`
- Scripts moved to `scripts/mesh/`
- Tests merged into `daemon/tests/`

## Action required
- Update any references from `~/GitHub/ConvergioMesh` to `~/GitHub/ConvergioPlatform/daemon/`
- The ConvergioMesh repo should be archived on GitHub with a README redirect
