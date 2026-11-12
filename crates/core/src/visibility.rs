//! Visibility controls for the six body parts and their two skin layers.

/// One of the six drawable body parts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BodyPartId {
    Head,
    Body,
    RightArm,
    LeftArm,
    RightLeg,
    LeftLeg,
}

/// Visibility of a body's inner and outer mesh.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PartVisibility {
    pub inner: bool,
    pub outer: bool,
}

impl Default for PartVisibility {
    fn default() -> Self {
        Self {
            inner: true,
            outer: true,
        }
    }
}

/// Visibility state for all six body parts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BodyVisibility {
    pub head: PartVisibility,
    pub body: PartVisibility,
    pub right_arm: PartVisibility,
    pub left_arm: PartVisibility,
    pub right_leg: PartVisibility,
    pub left_leg: PartVisibility,
}

impl Default for BodyVisibility {
    fn default() -> Self {
        let visible = PartVisibility::default();
        Self {
            head: visible,
            body: visible,
            right_arm: visible,
            left_arm: visible,
            right_leg: visible,
            left_leg: visible,
        }
    }
}

impl BodyVisibility {
    pub fn get(&self, part: BodyPartId) -> &PartVisibility {
        match part {
            BodyPartId::Head => &self.head,
            BodyPartId::Body => &self.body,
            BodyPartId::RightArm => &self.right_arm,
            BodyPartId::LeftArm => &self.left_arm,
            BodyPartId::RightLeg => &self.right_leg,
            BodyPartId::LeftLeg => &self.left_leg,
        }
    }

    pub fn get_mut(&mut self, part: BodyPartId) -> &mut PartVisibility {
        match part {
            BodyPartId::Head => &mut self.head,
            BodyPartId::Body => &mut self.body,
            BodyPartId::RightArm => &mut self.right_arm,
            BodyPartId::LeftArm => &mut self.left_arm,
            BodyPartId::RightLeg => &mut self.right_leg,
            BodyPartId::LeftLeg => &mut self.left_leg,
        }
    }

    pub fn set_part(&mut self, part: BodyPartId, inner: bool, outer: bool) {
        *self.get_mut(part) = PartVisibility { inner, outer };
    }

    pub fn set_inner_layers(&mut self, visible: bool) {
        for part in [
            BodyPartId::Head,
            BodyPartId::Body,
            BodyPartId::RightArm,
            BodyPartId::LeftArm,
            BodyPartId::RightLeg,
            BodyPartId::LeftLeg,
        ] {
            self.get_mut(part).inner = visible;
        }
    }

    pub fn set_outer_layers(&mut self, visible: bool) {
        for part in [
            BodyPartId::Head,
            BodyPartId::Body,
            BodyPartId::RightArm,
            BodyPartId::LeftArm,
            BodyPartId::RightLeg,
            BodyPartId::LeftLeg,
        ] {
            self.get_mut(part).outer = visible;
        }
    }

    pub fn set_all(&mut self, visible: bool) {
        self.set_inner_layers(visible);
        self.set_outer_layers(visible);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PARTS: [BodyPartId; 6] = [
        BodyPartId::Head,
        BodyPartId::Body,
        BodyPartId::RightArm,
        BodyPartId::LeftArm,
        BodyPartId::RightLeg,
        BodyPartId::LeftLeg,
    ];

    #[test]
    fn defaults_show_every_layer() {
        let visibility = BodyVisibility::default();
        for part in PARTS {
            assert_eq!(*visibility.get(part), PartVisibility::default());
        }
    }

    #[test]
    fn setting_one_part_does_not_change_other_parts() {
        let mut visibility = BodyVisibility::default();
        visibility.set_part(BodyPartId::Head, false, true);

        assert_eq!(
            visibility.get(BodyPartId::Head),
            &PartVisibility {
                inner: false,
                outer: true
            }
        );
        for part in PARTS.into_iter().filter(|part| *part != BodyPartId::Head) {
            assert_eq!(*visibility.get(part), PartVisibility::default());
        }
    }

    #[test]
    fn batch_operations_only_change_their_target_layer() {
        let mut visibility = BodyVisibility::default();
        visibility.set_outer_layers(false);
        assert!(PARTS.into_iter().all(|part| {
            let value = visibility.get(part);
            value.inner && !value.outer
        }));

        visibility.set_inner_layers(false);
        assert!(PARTS.into_iter().all(|part| {
            let value = visibility.get(part);
            !value.inner && !value.outer
        }));
    }

    #[test]
    fn set_all_is_idempotent() {
        let mut visibility = BodyVisibility::default();
        visibility.set_all(false);
        visibility.set_all(false);
        assert!(PARTS.into_iter().all(|part| {
            let value = visibility.get(part);
            !value.inner && !value.outer
        }));
    }
}
