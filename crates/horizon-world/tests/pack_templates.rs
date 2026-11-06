//! A world's data packs' structure templates (`data/<ns>/structure/*.nbt`)
//! are the ones jigsaw and template structures place, over the prepared
//! `worldgen/structure/` files, as vanilla's StructureTemplateManager lists
//! the packs' resources before the built-in ones.
use horizon_world::{
    DATA_VERSION,
    nbt::{self, Limits, NamedTag, Tag},
    registry::{BlockRegistry, BlockStateId},
    worldgen::{
        overrides::{WorldgenOverrides, template_id},
        structure::{BlockInfo, StateTransforms, StructureResources, save_nbt},
    },
};
use serde_json::json;
use std::{collections::BTreeMap, io::Write, path::Path};

fn registry() -> &'static BlockRegistry {
    let names = ["minecraft:air", "minecraft:stone"];
    let states = names
        .iter()
        .map(|name| Tag::compound([("id", Tag::String((*name).into()))]))
        .collect();
    let defaults = names
        .iter()
        .enumerate()
        .map(|(id, name)| ((*name).into(), Tag::Int(id as i32)))
        .collect();
    let registry = BlockRegistry::from_nbt(&Tag::compound([
        ("DataVersion", Tag::Int(DATA_VERSION)),
        ("states", Tag::List(states)),
        ("defaults", Tag::Compound(defaults)),
    ]))
    .unwrap();
    Box::leak(Box::new(registry))
}

fn transforms(states: usize) -> StateTransforms {
    let table = json!({
        "rotate": vec![json!([0, 0, 0]); states],
        "mirror": vec![json!([0, 0]); states],
        "full_block": vec![json!(true); states],
    });
    StateTransforms::from_json(&table.to_string(), states).unwrap()
}

/// A gzip NBT template of `size` with one stone block, as a pack holds it.
fn template_file(registry: &BlockRegistry, size: [i32; 3]) -> Vec<u8> {
    let stone = BlockInfo {
        pos: [0, 0, 0],
        state: BlockStateId(1),
        nbt: None,
    };
    let tag = save_nbt(
        registry,
        &transforms(registry.len()),
        size,
        vec![stone],
        &[],
        DATA_VERSION,
    );
    let raw = nbt::write_named(
        &NamedTag {
            name: "".into(),
            tag,
        },
        Limits::templates(),
    )
    .unwrap();
    let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
    gz.write_all(&raw).unwrap();
    gz.finish().unwrap()
}

#[test]
fn template_ids_are_the_packs_structure_files() {
    assert_eq!(
        template_id("data/terralith/structure/village/fortified/houses/a.nbt").as_deref(),
        Some("terralith:village/fortified/houses/a")
    );
    assert_eq!(
        template_id("data/minecraft/structure/woodland_mansion/1x1_a1.nbt").as_deref(),
        Some("minecraft:woodland_mansion/1x1_a1")
    );
    assert_eq!(template_id("data/terralith/structure/a.json"), None);
    assert_eq!(
        template_id("data/terralith/worldgen/structure/a.json"),
        None
    );
    assert_eq!(template_id("data/terralith/structure/.nbt"), None);
    assert_eq!(template_id("assets/terralith/structure/a.nbt"), None);
}

#[test]
fn the_highest_pack_template_is_placed_before_the_prepared_one() {
    let registry = registry();
    let prepared = tempfile_dir("prepared");
    // A prepared (vanilla) template, and a pack's file for the same id.
    let vanilla = prepared.join("minecraft/fossil");
    std::fs::create_dir_all(&vanilla).unwrap();
    std::fs::write(
        vanilla.join("spine_1.nbt"),
        template_file(registry, [1, 1, 1]),
    )
    .unwrap();

    let mut overrides = WorldgenOverrides::default();
    overrides.add_template(
        "minecraft:fossil/spine_1",
        template_file(registry, [2, 2, 2]),
    );
    overrides.add_template(
        "minecraft:fossil/spine_1",
        template_file(registry, [3, 3, 3]),
    );
    overrides.add_template("terralith:spire/top", template_file(registry, [5, 4, 5]));
    assert!(!overrides.is_empty());

    let resources = StructureResources::new(
        registry,
        transforms(registry.len()),
        &prepared,
        BTreeMap::new(),
        &BTreeMap::new(),
    );
    // Without the packs: the prepared file, and nothing for the pack's id.
    let plain = resources;
    assert_eq!(
        plain.template("minecraft:fossil/spine_1").unwrap().size,
        [1, 1, 1]
    );
    assert_eq!(
        plain.template("terralith:spire/top").unwrap().size,
        [0, 0, 0]
    );

    let resources = StructureResources::new(
        registry,
        transforms(registry.len()),
        &prepared,
        BTreeMap::new(),
        &BTreeMap::new(),
    )
    .with_pack_templates(overrides.templates());
    // The last pack's file wins over the lower pack's and the prepared one.
    let spine = resources.template("fossil/spine_1").unwrap();
    assert_eq!(spine.size, [3, 3, 3]);
    assert_eq!(&*spine.location, "minecraft:fossil/spine_1");
    // A template only a pack has loads instead of coming out empty.
    assert_eq!(
        resources.template("terralith:spire/top").unwrap().size,
        [5, 4, 5]
    );
    // Ids no pack has still fall back to the prepared folder.
    assert_eq!(
        resources.template("minecraft:missing").unwrap().size,
        [0, 0, 0]
    );
}

#[test]
fn templates_take_part_in_the_pack_set_key() {
    use std::hash::{BuildHasher, RandomState};
    let registry = registry();
    let mut a = WorldgenOverrides::default();
    let mut b = WorldgenOverrides::default();
    a.add_template("trek:a", template_file(registry, [1, 1, 1]));
    b.add_template("trek:a", template_file(registry, [1, 1, 1]));
    let hasher = RandomState::new();
    assert_eq!(a, b);
    assert_eq!(hasher.hash_one(&a), hasher.hash_one(&b));
    b.add_template("trek:a", template_file(registry, [2, 1, 1]));
    assert_ne!(a, b);
}

fn tempfile_dir(name: &str) -> std::path::PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("pack_templates-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// The palette name of a fixed template (26.3 writes `id`, older `Name`).
fn palette_names(tag: &Tag) -> Vec<String> {
    let Some(Tag::List(palette)) = tag.get("palette") else {
        return Vec::new();
    };
    palette
        .iter()
        .filter_map(|s| match s.get("id").or_else(|| s.get("Name")) {
            Some(Tag::String(name)) => Some(name.to_string_lossy()),
            _ => None,
        })
        .collect()
}

#[test]
fn an_old_packs_template_is_upgraded_like_vanilla_reads_it() {
    // A 1.20.1 (3465) template naming blocks renamed since: `grass` became
    // `short_grass` (1.20.3) and `chain` became `iron_chain` (1.21.9).
    let state = |name: &str| Tag::compound([("Name", Tag::String(name.into()))]);
    let mut tag = Tag::compound([
        ("DataVersion", Tag::Int(3465)),
        ("size", Tag::List(vec![Tag::Int(1); 3])),
        (
            "palette",
            Tag::List(vec![
                state("minecraft:grass"),
                state("minecraft:chain"),
                state("minecraft:stone"),
            ]),
        ),
        ("blocks", Tag::List(Vec::new())),
        ("entities", Tag::List(Vec::new())),
    ]);
    horizon_world::datafix::update_structure(&mut tag);
    assert_eq!(
        palette_names(&tag),
        [
            "minecraft:short_grass",
            "minecraft:iron_chain",
            "minecraft:stone"
        ]
    );
    // A current template is left as it is.
    let mut current = Tag::compound([
        ("DataVersion", Tag::Int(DATA_VERSION)),
        ("palette", Tag::List(vec![state("minecraft:grass")])),
    ]);
    horizon_world::datafix::update_structure(&mut current);
    assert_eq!(palette_names(&current), ["minecraft:grass"]);
}
