//! The friendly name Irori gives a Claude session nobody named (card #21;
//! I:internal/names, coord.md §3.13): a short word from a fixed list,
//! distinct from every live agent's name.

use gocompat::fnv::fnv32a;

/// The list names come from (card #224: about a thousand, so a name comes
/// back weeks later, not the next day). Every word is 3-8 lowercase
/// letters, and none is the start of another, so an @mention of one never
/// reads as another. Append only: a new session's first choice is its
/// session id's hash over this list. A session keeps the name it was given
/// (names.json), so a longer list never renames a running agent.
pub const WORDS: &[&str] = &[
    "acorn", "alder", "amber", "anchor", "aspen", "aster", "badger", "basil", "beacon", "birch",
    "bison", "bramble", "brook", "cedar", "cinder", "clover", "comet", "coral", "cove", "crane",
    "cricket", "cypress", "dahlia", "delta", "dune", "eagle", "ember", "fennel", "fern", "finch",
    "fjord", "flint", "forest", "fox", "garnet", "gecko", "ginger", "glacier", "grove", "harbor",
    "hazel", "heron", "hickory", "holly", "ibis", "indigo", "iris", "ivy", "jade", "jasper",
    "juniper", "kestrel", "kite", "koala", "lagoon", "lark", "laurel", "lemur", "lichen", "lily",
    "linden", "lotus", "lynx", "magpie", "maple", "marble", "marten", "meadow", "mesa", "mink",
    "minnow", "moss", "nectar", "newt", "nimbus", "oak", "ocelot", "olive", "onyx", "orca",
    "osprey", "otter", "owl", "panda", "pebble", "pelican", "pepper", "pine", "plover", "poppy",
    "prairie", "puffin", "quail", "quartz", "raven", "reed", "ridge", "robin", "rowan", "saffron",
    "sage", "salmon", "sequoia", "shale", "sparrow", "spruce", "stork", "summit", "swift", "tansy",
    "tern", "thistle", "thyme", "tiger", "topaz", "tundra", "tulip", "umber", "valley", "vireo",
    "walnut", "willow", "wren", "yarrow", "yew", "zephyr", "zinnia", "beech", "fir", "larch",
    "poplar", "sumac", "acacia", "baobab", "banyan", "balsa", "cassia", "catalpa", "mahogany",
    "mangrove", "myrtle", "palmetto", "quince", "redwood", "teak", "tamarind", "tupelo", "yucca",
    "ironwood", "boxwood", "buckeye", "chestnut", "dogwood", "hemlock", "locust", "mulberry",
    "hawthorn", "sycamore", "tamarack", "wisteria", "magnolia", "apricot", "pawpaw", "medlar",
    "loquat", "guava", "lychee", "mango", "papaya", "kumquat", "citron", "pomelo", "carob",
    "kapok", "eucalypt", "kauri", "rimu", "totara", "manuka", "kowhai", "azalea", "begonia",
    "camellia", "crocus", "freesia", "gardenia", "geranium", "hyacinth", "jasmine", "lavender",
    "lilac", "marigold", "orchid", "peony", "petunia", "primrose", "snowdrop", "bluebell",
    "cosmos", "heather", "hibiscus", "lupin", "phlox", "sorrel", "trillium", "verbena", "anemone",
    "arnica", "borage", "calla", "clematis", "daphne", "fuchsia", "gentian", "gorse", "hosta",
    "jonquil", "kalmia", "lantana", "lobelia", "lunaria", "mimosa", "nigella", "oleander",
    "oxalis", "protea", "salvia", "scilla", "sedum", "statice", "tuberose", "valerian", "vetch",
    "viola", "xenia", "allium", "alyssum", "astilbe", "bergenia", "briar", "canna", "celosia",
    "clary", "cyclamen", "dianthus", "erica", "gerbera", "godetia", "ixia", "liatris", "malva",
    "nerine", "nemesia", "orris", "pansy", "plumbago", "quamash", "scabious", "thrift", "tithonia",
    "ursinia", "vinca", "weigela", "anise", "caraway", "cardamom", "chervil", "chicory", "chive",
    "cilantro", "cumin", "dill", "garlic", "hyssop", "lovage", "marjoram", "nutmeg", "oregano",
    "paprika", "parsley", "rosemary", "savory", "sesame", "tarragon", "turmeric", "wasabi",
    "angelica", "bayleaf", "burdock", "catnip", "comfrey", "costmary", "dittany", "galangal",
    "licorice", "mugwort", "nettle", "perilla", "purslane", "shiso", "stevia", "woodruff",
    "zaatar", "avocet", "bittern", "bluejay", "bobolink", "bunting", "buzzard", "canary",
    "cardinal", "condor", "coot", "cowbird", "cuckoo", "curlew", "dipper", "dove", "dunlin",
    "egret", "eider", "emu", "falcon", "flamingo", "gannet", "godwit", "goose", "grackle", "grebe",
    "grouse", "gull", "harrier", "hawk", "hoopoe", "hornbill", "jackdaw", "junco", "kakapo", "kea",
    "kingbird", "lapwing", "linnet", "loon", "lorikeet", "macaw", "mallard", "merlin", "moorhen",
    "mynah", "nuthatch", "oriole", "ostrich", "ouzel", "parakeet", "peacock", "penguin", "petrel",
    "pheasant", "phoebe", "pigeon", "pipit", "pochard", "quetzal", "redpoll", "redstart", "shrike",
    "siskin", "skua", "skylark", "snipe", "starling", "sunbird", "swallow", "swan", "tanager",
    "thrush", "titmouse", "toucan", "towhee", "turaco", "veery", "wagtail", "warbler", "waxwing",
    "weaver", "whimbrel", "wigeon", "woodcock", "bulbul", "drongo", "fulmar", "gadwall", "jacana",
    "kakariki", "motmot", "oxpecker", "pewee", "pitta", "potoo", "prion", "scaup", "scoter",
    "smew", "tui", "tody", "trogon", "whydah", "aardvark", "alpaca", "antelope", "axolotl",
    "baboon", "beaver", "bobcat", "bongo", "capybara", "caribou", "chamois", "cheetah", "chipmunk",
    "civet", "coati", "cougar", "coyote", "dingo", "dormouse", "duiker", "eland", "elk", "ermine",
    "ferret", "fennec", "gazelle", "gerbil", "gibbon", "giraffe", "gnu", "gopher", "gorilla",
    "hamster", "hedgehog", "hippo", "ibex", "impala", "jackal", "jaguar", "jerboa", "kangaroo",
    "kinkajou", "kudu", "lemming", "leopard", "llama", "loris", "manatee", "mandrill", "marmot",
    "meerkat", "mongoose", "moose", "muskox", "narwhal", "numbat", "okapi", "opossum", "oryx",
    "pangolin", "panther", "peccary", "pika", "platypus", "possum", "puma", "quokka", "quoll",
    "raccoon", "reindeer", "rhino", "sable", "serval", "shrew", "sloth", "stoat", "tapir",
    "tarsier", "tenrec", "vicuna", "vole", "wallaby", "walrus", "warthog", "weasel", "wildcat",
    "wombat", "yak", "zebra", "zebu", "caracal", "dhole", "fossa", "galago", "genet", "grison",
    "hutia", "margay", "nilgai", "nyala", "oncilla", "pudu", "saiga", "takin", "tayra", "urial",
    "wapiti", "zorilla", "abalone", "anchovy", "barnacle", "beluga", "bonito", "catfish", "clam",
    "cockle", "cod", "conch", "dolphin", "dory", "eel", "flounder", "grouper", "haddock", "hake",
    "halibut", "herring", "kelp", "krill", "limpet", "lobster", "mackerel", "marlin", "mollusk",
    "monkfish", "mussel", "nautilus", "octopus", "oyster", "piranha", "plaice", "pollock", "prawn",
    "remora", "sardine", "scallop", "shark", "shrimp", "snapper", "sponge", "squid", "starfish",
    "sturgeon", "sunfish", "tarpon", "tilapia", "trout", "tuna", "turbot", "urchin", "wahoo",
    "whelk", "wrasse", "zander", "brill", "gudgeon", "guppy", "loach", "mullet", "nori", "orfe",
    "tench", "walleye", "bluefin", "cobia", "grunion", "opah", "pompano", "beetle", "cicada",
    "hornet", "katydid", "ladybug", "mantis", "mayfly", "midge", "moth", "scarab", "wasp",
    "caddis", "lacewing", "sawfly", "stonefly", "earwig", "adder", "anole", "boa", "caiman",
    "gharial", "iguana", "krait", "mamba", "skink", "tortoise", "tuatara", "agama", "agate",
    "amethyst", "azurite", "basalt", "beryl", "bismuth", "calcite", "citrine", "cobalt",
    "corundum", "diamond", "dolomite", "emerald", "feldspar", "fluorite", "galena", "gneiss",
    "granite", "graphite", "gypsum", "hematite", "iolite", "jacinth", "kyanite", "lapis", "mica",
    "obsidian", "opal", "peridot", "platinum", "pumice", "pyrite", "ruby", "rutile", "sapphire",
    "schist", "spinel", "sulfur", "talc", "titanium", "zircon", "chert", "cinnabar", "diorite",
    "gabbro", "mercury", "nephrite", "pewter", "porphyry", "scoria", "tektite", "tufa", "zeolite",
    "argon", "xenon", "krypton", "boron", "cerium", "erbium", "indium", "iridium", "osmium",
    "rhodium", "yttrium", "aurora", "blizzard", "breeze", "cirrus", "cyclone", "drizzle",
    "monsoon", "rainbow", "sleet", "squall", "sunbeam", "sunrise", "sunset", "thunder", "tornado",
    "twilight", "typhoon", "chinook", "mistral", "sirocco", "foehn", "halo", "stratus", "cumulus",
    "sundog", "vapor", "rime", "graupel", "flurry", "downpour", "antares", "aquila", "arcturus",
    "aries", "auriga", "bootes", "callisto", "capella", "carina", "castor", "cepheus", "cetus",
    "corvus", "cygnus", "deneb", "draco", "europa", "galaxy", "gemini", "halley", "hydra",
    "jupiter", "kepler", "libra", "lyra", "meteor", "mira", "nebula", "neptune", "nova", "orion",
    "pavo", "pegasus", "perseus", "phoenix", "pluto", "polaris", "pollux", "procyon", "pulsar",
    "quasar", "regulus", "rigel", "saturn", "sirius", "spica", "triton", "uranus", "vega", "virgo",
    "zenith", "equinox", "solstice", "altair", "mizar", "alcor", "ariel", "oberon", "phobos",
    "deimos", "dione", "tethys", "umbriel", "larissa", "naiad", "thalassa", "despina", "proteus",
    "nereid", "arroyo", "atoll", "bayou", "bluff", "boulder", "butte", "canyon", "cavern", "cliff",
    "crag", "crater", "desert", "estuary", "ford", "gorge", "glade", "gulch", "highland", "hollow",
    "inlet", "island", "isthmus", "knoll", "ledge", "moraine", "oasis", "outcrop", "pampas",
    "plateau", "rapids", "ravine", "reef", "savanna", "scree", "shoal", "steppe", "strait",
    "stream", "tarn", "terrace", "thicket", "volcano", "wetland", "geyser", "grotto", "cairn",
    "loch", "esker", "drumlin", "kame", "karst", "sinkhole", "caldera", "cirque", "couloir",
    "scarp", "talus", "wadi", "playa", "sabkha", "avocado", "bagel", "banana", "biscuit",
    "brioche", "caramel", "cashew", "cocoa", "coconut", "crepe", "crumpet", "currant", "custard",
    "dumpling", "eggplant", "fudge", "granola", "grape", "kale", "leek", "lentil", "macaron",
    "melon", "millet", "mochi", "muffin", "nougat", "okra", "pancake", "parsnip", "peanut",
    "popcorn", "pretzel", "prune", "pumpkin", "quinoa", "raisin", "ramen", "risotto", "scone",
    "sorbet", "spinach", "sprout", "strudel", "sushi", "toffee", "tofu", "truffle", "turnip",
    "waffle", "yam", "yogurt", "cobbler", "crumble", "dango", "gnocchi", "kimchi", "latte",
    "matcha", "miso", "panko", "pesto", "polenta", "ravioli", "tahini", "tamale", "udon", "wonton",
    "gelato", "biscotti", "brownie", "cupcake", "donut", "eclair", "flan", "mousse", "praline",
    "torte", "alps", "andes", "avalon", "azores", "borneo", "danube", "everest", "ganges", "nile",
    "sahara", "tahiti", "yukon", "canoe", "chime", "compass", "crystal", "cymbal", "easel",
    "fable", "feather", "fiddle", "flute", "forge", "fountain", "gable", "gadget", "gazebo",
    "glider", "goblet", "harp", "helm", "hammock", "kayak", "lantern", "lattice", "locket", "loom",
    "lute", "mantle", "mosaic", "oar", "orbit", "paddle", "pagoda", "palette", "parasol",
    "pendant", "piano", "prism", "pulley", "quill", "quiver", "raft", "rudder", "sail", "satchel",
    "scroll", "sextant", "sickle", "spindle", "spire", "spur", "stencil", "tambour", "teapot",
    "thimble", "tiller", "timber", "trellis", "trumpet", "tuba", "turret", "vase", "velvet",
    "violin", "windmill", "yacht", "yoke", "zither", "banjo", "cello", "cornet", "dulcimer",
    "gong", "lyre", "mandolin", "marimba", "oboe", "ocarina", "piccolo", "sitar", "tabla",
    "ukulele", "kazoo", "azure", "cerise", "crimson", "ecru", "fawn", "henna", "ochre", "puce",
    "russet", "sepia", "sienna", "taupe", "viridian",
];

/// The name for `seed` (a session id): the word its hash lands on, else the
/// next word along that `taken` refuses, wrapping round; when every word is
/// taken, the same probe with `2`, then `3`, ... appended. `taken` is asked
/// with the lowercase name.
pub fn pick(seed: &str, taken: Option<&dyn Fn(&str) -> bool>) -> String {
    let n_words = WORDS.len();
    let start = (fnv32a(seed.as_bytes()) % n_words as u32) as usize;
    let mut n = 1u64;
    loop {
        for i in 0..n_words {
            let mut name = WORDS[(start + i) % n_words].to_string();
            if n > 1 {
                name.push_str(&n.to_string());
            }
            match taken {
                Some(t) if t(&name) => {}
                _ => return name,
            }
        }
        n += 1;
    }
}

/// The first word for `seed` that `taken` doesn't refuse, probing the list
/// once from the word its hash lands on; `None` when every word is taken
/// (then [`pick`] goes on to numbered names).
pub fn pick_free(seed: &str, taken: &dyn Fn(&str) -> bool) -> Option<String> {
    let n_words = WORDS.len();
    let start = (fnv32a(seed.as_bytes()) % n_words as u32) as usize;
    (0..n_words)
        .map(|i| WORDS[(start + i) % n_words])
        .find(|w| !taken(w))
        .map(str::to_string)
}

/// The form names are compared in: `strings.ToLower(strings.TrimSpace(name))`.
pub fn fold(name: &str) -> String {
    gocompat::text::to_lower_str(trim_space(name))
}

/// `strings.TrimSpace`.
pub(crate) fn trim_space(s: &str) -> &str {
    s.trim_matches(gocompat::text::is_space)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn words_are_many_distinct_short_and_prefix_free() {
        assert!(WORDS.len() >= 1000, "only {} words", WORDS.len());
        let mut seen = HashSet::new();
        for w in WORDS {
            assert!(seen.insert(*w), "{w} twice");
            assert!(
                (3..=8).contains(&w.len()) && w.bytes().all(|b| b.is_ascii_lowercase()),
                "{w}: not 3-8 lowercase letters"
            );
        }
        let mut sorted: Vec<&str> = WORDS.to_vec();
        sorted.sort_unstable();
        // In sorted order a word that starts another sits right before one
        // that starts with it.
        for pair in sorted.windows(2) {
            assert!(
                !pair[1].starts_with(pair[0]),
                "{} is the start of {}",
                pair[0],
                pair[1]
            );
        }
    }

    #[test]
    fn pick_free_probes_once() {
        let first = pick_free("s1", &|_| false).unwrap();
        assert_eq!(first, pick("s1", None));
        let second = pick_free("s1", &|w| w == first).unwrap();
        assert_ne!(second, first);
        assert_eq!(pick_free("s1", &|_| true), None);
    }
}
