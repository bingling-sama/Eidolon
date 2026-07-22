# Eidolon Rendering Context

This glossary names the player appearance concepts shared by the library, CLI, and browser adapter.

## Language

**Character**:
A player-aligned appearance with a posture and world-space placement. It contains at least a Body or Back equipment, and may also carry a nametag.
_Avoid_: Model, actor

**Body**:
The optional visible player body, combining a Skin with classic or slim arm geometry.
_Avoid_: Player model, skinned mesh

**Skin**:
The square visual atlas that supplies a Body's body and clothing appearance.
_Avoid_: skin texture, player image

**Posture**:
The complete pose of a Character, including body joints and the angles used by any selected back equipment.
_Avoid_: Pose settings, equipment angles

**Back equipment**:
Optional equipment attached to a Character's back. It has one Back equipment atlas and is presented as either a Cape or Elytra.
_Avoid_: cape flag, elytra flag, back attachment

**Back equipment atlas**:
The visual atlas shared by the Cape and Elytra presentations of the same Back equipment.
_Avoid_: cape texture, elytra texture

**Cape**:
A hanging presentation of Back equipment that rotates around the upper-back attachment edge.
_Avoid_: cloak

**Elytra**:
A winged presentation of Back equipment: a mirrored pair sharing an upper-back attachment that can open into a flight pose.
_Avoid_: wings, glider
