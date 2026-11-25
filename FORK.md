# Fernrohr's gpui-kit fork

The `fernrohr/<upstream-tag>` branches are upstream gpui-kit releases plus the patches the
[Fernrohr](https://github.com/pilgrimagesoftware/Fernrohr-App) app needs before upstream ships them.
Fernrohr builds against a tag on that branch through `[patch.crates-io]`, covering all five crates
(`gpui-kit`, `gpui-component`, `gpui-base`, `gpui-component-macros`, `gpui-kit-assets`).

## Patches

| Patch | Files | Drop it when |
| --- | --- | --- |
| Menu hover doesn't change what Enter activates ([Fernrohr#118](https://github.com/pilgrimagesoftware/Fernrohr/issues/118)) | `crates/component/src/menu/popup_menu.rs` (`render_item`'s `on_hover`) | Upstream stops moving `selected_index` on hover over a plain item |

## Moving to the next upstream release

1. Fetch the tag: `git fetch https://github.com/longbridge/gpui-kit.git refs/tags/vX.Y.Z:refs/tags/vX.Y.Z`
2. Branch from it: `git switch -c fernrohr/vX.Y.Z vX.Y.Z`
3. Re-apply each patch above: `git cherry-pick <commit>` from the previous `fernrohr/` branch, and check its
   "drop it when" condition first. The hover handler moves between releases, so expect to port it by hand.
4. Copy this file across, update the table, commit, push the branch, and tag `vX.Y.Z-fernrohr.1`.
5. In Fernrohr, point the five `[patch.crates-io]` entries at the new tag and bump `gpui-kit` to `=X.Y.Z`.
