use hotplug::{malleable, malleable_detours};

#[test]
fn detours() {
	malleable_detours.add_plug(|a, b, next| {
		dbg!("First!");
		next(a, b)
	});

	malleable_detours.add_plug(|a, b, next| {
		dbg!("Second!");
		next(a, b)
	});

	malleable((), &());
}
