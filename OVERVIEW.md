Below is a **developer‑focused teardown** of *Imperialism* (1997), organized so you can re‑build the whole machine in **Bevy + Rust** without guessing what the original systems were doing. Where a rule is specific to the 1997 game, I cite the manual.

---

## 0) Core turn loop & screens

**Screens / major modes**

* **Terrain / Map** (global state, unit orders, construction).
* **Transport** (allocate rail/port capacity across commodities).
* **Industry (City)** (production, workforce, unit construction).
* **Trade** (place bids/offers; merchant marine capacity matters).
* **Diplomacy** (relations, treaties, overtures, subsidies/boycotts).

**Order of resolution when you hit “End Turn”**

1. **Diplomatic offers** exchanged → accepted/rejected
2. **Trade deals** offered → you accept/reject
3. **Industrial production** resolves
4. **Military conflicts** resolve
5. **Intercepted / blockaded trades** are cancelled
6. **All transported commodities** land in warehouse for next turn

Implement this as a single authoritative **TurnSystem** that drains per‑screen command buffers in that order.

---

## 1) World model

### Countries & victory

* Two country types: **Great Powers** (playable) and **Minor Nations** (non‑playable); random maps ship with **7 Great Powers** and **16 Minors**. Minors can be conquered/colonized; they never win.
* **Provinces**: world is partitioned into provinces; random worlds have **120 provinces**; each Great Power starts with **8**, each Minor with **4**. Each province has exactly one **capital or town**. Provinces have a single owner at turn end; they’re taken, not split.
* **Win condition**: global **Council of Governors** vote every ~10 years; world nominates two Great Powers; majority vote wins.

### Map & tiles (what matters for systems)

* Tiles carry **terrain** and optional **resource opportunity**. Some resources are visible from terrain (e.g., cotton on plantation); **minerals (coal/iron/gold/gems, later oil)** must be **discovered by a Prospector**.
* A resource tile only contributes if it’s **connected** to your network (capital or port) via **rail + depot/port** (details in §3).

---

## 2) Civilians & terrain development

**Civilian unit types** (one action/turn, placed on tiles):

* **Prospector** — reveals **minerals** (coal/iron/gold/gems) on barren hills/mountains; after **Oil Drilling** tech, can also prospect swamps/deserts/tundra for **oil**.
* **Miner** — opens & upgrades **mines** (Lv I→II via **Square‑Set Timbering**, Lv II→III via **Dynamite**). **Gold/gems** produce **1/unit per mine level**, **coal/iron** produce **double** that (max 6/unit at Lv III).
* **Farmer, Rancher, Forester, Driller** — improve **grain/fruit/cotton**, **wool/livestock**, **timber**, **oil** respectively; per‑tile output rises with development level (see table).
* **Engineer** — builds **rails**, **depots**, **ports**, **fortifications**; rail placement is adjacency based; some terrain requires tech to rail into.
* **Developer** — **only works in Minor Nations**: buys resource tiles abroad once relations are high enough (get a Developer “reward” via embassy/pacts/trade/bribes), then your other civilians can improve those tiles **in the minor**. You earn **overseas profits** when those resources are sold on the world market; the share scales up to **100%** with relationship level.

**Baseline per‑tile output by development level** (original values to keep balance):

* Food/fiber/timber at Lv0=1 → Lv1=2 → Lv2=3 → Lv3=4 per turn
* **Coal/iron** at Lv0=0 → Lv1=2 → Lv2=4 → Lv3=6
* **Gold/gems** at Lv0=0 → Lv1=1 → Lv2=2 → Lv3=3
* **Oil** at Lv0=0 → Lv1=2 → Lv2=4 → Lv3=6

**Colonies vs conquest (economic angle)**

* **Conquered provinces** act like your own: you can rail/port/fort them; resources join your network as normal. **Colonies** (won diplomatically/defensively) remain semi‑independent: their outputs are sold on the world market; you get **right of first refusal** and the **profits** flow via the “overseas profits” mechanic.

---

## 3) Transport network (rails, depots, ports) & capacity

**What the network is**

* **Resources** flow from connected tiles to **Industry** at end of turn. Early game, only tiles adjacent to the capital count; extending network requires the **Engineer** to build **rails/depots/ports**.

**Depots & ports: how gathering works**

* A **Depot or Port** gathers **all commodities** in **its own tile + all adjacent tiles** (8‑neighborhood). Only **one** facility gathers a tile; space them at least **two tiles apart** to avoid duplication. Depots/ports must be **connected** to count.
* **Connecting a Depot** means there is a **rail path** from that depot to the **capital** (optionally via a **port** and then sea). Unconnected = dead. A depot shows a **two‑light signal**: **green** connected, **red** not.
* **Ports** can be **coastal** or **river**; generally easier to “connect” (don’t need rails to capital), but a **river port loses connection** if you lose a downstream province; **sea ports lose connection** if the adjacent **sea zone** is under **undisputed** enemy naval control.

**Capacity model**

* You have a national **Transport Capacity** (raised in **Railyard** on the Industry screen). The **Transport screen** lets you assign capacity **per commodity** via sliders; only commodities currently available are active.
* **Moving regiments by rail** is limited by this same capacity, at **5 capacity per “armaments point”** of the regiment. You don’t have to steal capacity back from the Transport sliders to rail‑move an army, but the total system cap still limits how many regiment‑points you can move this turn.

**Town development via being connected**

* Each owned province has a **town**. If you place a **connected** depot/port on/next to it, the town gradually starts producing **materials** (steel/lumber/fabric) and later **goods** (furniture/clothing/consumer goods). Max **goods** throughput = **½ materials** throughput. **Gold** adds cash; **gems** add more; **horses** are only for cav/artillery.

---

## 4) Industry screen (production, workforce, units)

**Production economies are strictly 2:1 transforms**

* **Resources → Materials → Goods** with **2:1 ratios at each step**.

    * **Textiles**: 2×(cotton/wool) → 1×fabric; 2×fabric → 1×clothing
    * **Wood**: 2×timber → 1×(lumber|paper); 2×lumber → 1×furniture
    * **Metal**: 1×iron + 1×coal → 1×steel; 2×steel → 1×(hardware|armaments)
    * **Oil**: 2×oil → 1×fuel
    * **Food**: (2 grain + 2 livestock/fish + 2 fruit) → 2×canned food
    * **Horses**: produced via the horse economy building
      (Exact table shown in manual, but above is the same semantics.)

**Workforce & training**

* Workers supply **Labour**: **1 / 2 / 4** points per **untrained / trained / expert** worker. Train at **Trade School** (costs **paper + cash**) and the worker is removed from labour for that turn.
* **Food consumption**: every worker eats **1 raw food** with a preference rotation (grain / fruit / livestock|fish / repeat). If the preferred type isn’t available, they will eat **canned food**; otherwise they get **sick** (0 labour). With **no food** they **die**.
* **Migration (Capitol building)** recruits **untrained workers** if you can supply **canned food + clothing + furniture**. Recruit cap per turn = **⌊provinces/4⌋** early; later an upgrade raises the cap to **⌊provinces/3⌋**.
* **Power** (from **Power Plant** after Oil tech) adds **labour** for the turn; it isn’t a commodity and consumes **fuel** to boost available labour.

**Industry capacity & costs**

* Build **mills**/**factories** at specific sites; initial capacity: **mills start at 2**, **factories at 1**, and each capacity point costs **1 lumber + 1 steel**. You can **expand** later at same per‑cap cost.

**Shipyard**

* Builds **merchant ships** (adds cargo holds / trade capacity and average speed) and **warships** (see §6). Ships don’t consume labour but do consume **lumber/steel** (and **armaments** for warships). Over‑building early will choke other industries.

---

## 5) Trade & the world market

**How it runs**

* Each turn you place **Offers** (sell) and **Bids** (buy). At resolution you receive sequential **offers to buy/sell**; you accept/reject each. Purchased goods show up next turn in **Industry**; sold goods leave the **Warehouse** immediately.
* **Merchant marine capacity** is a hard limit on how many deals you can accept; escorts and interceptions kick in during war (see §6). **Offers resolve in a fixed commodity order**, so you may want to hold capacity for late‑order commodities (e.g., iron).
* The **Trade Book** shows, per commodity, who’s selling/buying and the **ranked order** of bidders; you must have **placed a bid** to review that market (“market presence”).
* **Deal Book** at end of the trade phase summarizes **sales/purchases**, **overseas profits**, **military upkeep**, and **credit** status. Go past your **credit limit** and you risk **forced sales** of warehouse goods at lousy prices.

**Minor Nations & your overseas work**

* When you **buy land** in a Minor and improve it, extra production is **sold by the Minor** on the world market; **you get a cash share** at end of trade (your **overseas profits**). Relations → higher share.

---

## 6) Diplomacy

* **Info view** shows industrial/military size; for **Minors** you also see **favorite trading partner** and the **Great Power with best relations**; subsidies/boycotts are visible overlays.
* **Overtures** include treaties, **war/peace**, embassies, etc. **War** can’t be refused; effects apply from the next turn. Declarations shift global opinion.
* **Trade policy tools**: **subsidies** (pay above market, sell below market, improving relations) and **boycotts** (no trade). These are set by Great Powers.
* **Council of Governors** tab shows province‑by‑province votes each session—the meta‑goal of the campaign.

---

## 7) Land & naval forces

**Regiments, eras, and upgrades**

* 27 regiment types across **three eras** (~1815–45, ~1845–80, ~1880+). New tech **replaces** old buildable types; you can **upgrade** existing regiments in the **Garrison Book** when tech is present.
* **Upgrade requirements** per category (examples):
  Light Infantry: Skirmishers → (Bessemer Converter) Sharpshooters → (Machine Guns) Rangers;
  Heavy Cav: Cuirassiers → (Breechloading Rifle) Carbine Cavalry → (Internal Combustion) Armour. (Full table in manual.)
* **Militia** are free, province‑bound defenders that auto‑upgrade with tech; they cannot leave their home province.

**Movement & rail**

* **March** to adjacent friendly province; **Train** to non‑adjacent friendly province within your borders (consumes rail **capacity limit**, see §3). To attack, your forces must be **adjacent**; declare **war** first. **Sea landings** are risky; fleets can be intercepted.

**Experience & morale**

* Regiments gain **medals** by fighting; each medal boosts **initiative** and **firepower**; **4 medals ≈ 2×** the power of the base unit.

**Entrenchments & forts**

* Provinces garrisoned are **entrenched** by default (20% damage reduction). **Forts** (built by Engineer) layer three levels; each adds **10%** (best fort = **50%** total with entrenchment). Fort walls can only be bypassed/destroyed by **Combat Engineers** or heavy guns.

**Navy & merchant marine**

* **Merchant ships**: add cargo **capacity** and raise average **speed** (harder to intercept). War at sea can **intercept/sink** merchants; assign warships as **escorts**. Damaged warships must **repair in port**. Obsolete classes are **scrapped**, spreading XP.
* Warships are “fast” (frigates/raiders/armoured cruisers/battlecruisers) vs “battle” (ships‑of‑the‑line/ironclads/advanced ironclads/dreadnoughts). **Range** is the dominant combat stat.
* **Naval control** of a sea zone can **blockade ports** (breaks their connection; see §3).

---

## 8) Technology

* Techs are global inventions; when they appear, any nation can **invest cash** to unlock; **effects** include: unlocking new regiment types, enabling new **mine/rail/construction** abilities (e.g., Oil Drilling, Square‑Set Timbering, Dynamite), power generation, etc. You can cancel the investment before End Turn.

---

## 9) Suggested Bevy ECS mapping

Below is a **lean, data‑driven** mapping that keeps rules declarative and screens thin.

### Data types (Rust‑ish)

```rust
// World / geography
struct TileId(u32);
struct ProvinceId(u16);
struct CountryId(u8);

#[derive(Clone, Copy)]
enum Terrain { Plains, Hills, Mountains, Forest, Desert, Swamp, Tundra, Coast, River, /* ... */ }

#[derive(Clone, Copy)]
enum Commodity {
    Grain, Fruit, Livestock, Fish,
    Cotton, Wool, Timber,
    Coal, Iron, Gold, Gems, Oil,
    Fabric, Paper, Lumber, Steel, Fuel,
    Hardware, Armaments, Clothing, Furniture, CannedFood,
    Horses,
}

#[derive(Clone, Copy)]
enum ResourceDev { Lv0, Lv1, Lv2, Lv3 }

#[derive(Component)]
struct Tile {
    id: TileId,
    province: ProvinceId,
    owner: Option<CountryId>,
    terrain: Terrain,
    // e.g. Some((Commodity::Coal, discovered: bool, dev: ResourceDev))
    resource: Option<(Commodity, bool, ResourceDev)>,
    town: bool,
    capital: bool,
}

#[derive(Component)]
struct Depot { connected: bool }
#[derive(Component)]
struct Port  { connected: bool, is_river: bool }

#[derive(Component)]
struct Rail; // edge component on Tile graph; build via Engineer

// Ownership / political
struct Province {
    id: ProvinceId,
    owner: CountryId,
    adjacent: SmallVec<[ProvinceId; 8]>,
    town_tile: TileId,
    garrison: Vec<Entity>, // regiments
}

struct Country {
    id: CountryId,
    great_power: bool,
    treasury: i64,
    credit_limit: i64,
    merchant_marine: MerchantMarine,
    transport_capacity: u32,
    tech: FxHashSet<TechId>,
    relations: FxHashMap<CountryId, Relation>,
    warehouse: FxHashMap<Commodity, i32>,
    workforce: Workforce,
    industries: Industries,
}

struct MerchantMarine { holds: u32, avg_speed: u8 }
struct Workforce { untrained: u32, trained: u32, expert: u32 }
struct Industries {
    lumber_mill_cap: u32, steel_mill_cap: u32, textile_mill_cap: u32,
    armory_cap: u32, hardware_cap: u32, clothing_cap: u32, furniture_cap: u32,
    paper_mill_cap: u32, food_proc_cap: u32, horse_ranch_cap: u32,
    railyard_level: u8, shipyard_level: u8, power_plant_level: u8,
}

// Civilians
enum CivilianKind { Prospector, Miner, Farmer, Rancher, Forester, Driller, Engineer, Developer }

#[derive(Component)]
struct Civilian {
    kind: CivilianKind,
    location: TileId,
    orders: Option<CivilianOrder>,
}

// Military
#[derive(Clone, Copy)]
enum RegimentCategory { Militia, LightInf, RegularInf, HeavyInf, LightCav, HeavyCav, LightArt, HeavyArt, Engineers }

#[derive(Component)]
struct Regiment {
    country: CountryId,
    province: ProvinceId,
    category: RegimentCategory,
    era_type: u8,           // e.g., 0/1/2 within category
    armaments_points: u8,   // drives rail move cost (5×)
    medals: u8,             // 0..4
    entrenched: bool,
}

// Tech
struct Tech { id: TechId, cost: i64, effects: Vec<TechEffect>, appears_year: i32 }
```

### Declarative rule tables

Drive as much as possible from tables:

* **ResourceOutput[Commodity][DevLevel] → per‑turn units** (use the values from §2).
* **Transforms** (2:1): list **inputs**, **outputs**, **capacity site**.
* **TransportOrder** for Trade resolution (to match offer order).
* **RegimentUpgrade[Category]** with **required techs per era**.

### Systems (Bevy)

* `CivilianWorkSystem`: executes Prospector/Miner/Farmer/... actions, bumps tile `resource.discovered/dev`, emits construction on Engineer orders, enforces “adjacent depot/port or capital” for counting outputs.
* `NetworkConnectSystem`: BFS from capital and ports over **Rail** edges to mark **Depot/Port.connected** and produce **GatherZones** (tile + neighbors). Break connections if province along the line is lost or port is blockaded.
* `TransportAllocationSystem`: applies **Transport screen** sliders to generate per‑commodity quotas; at end‑turn drain capacity into **Industry input queues**; compute “rail budget” available for army moves (5× armaments).
* `TownGrowthSystem`: if town adjacent to connected depot/port, increment its **materials** and then **goods** capacities with the 1:2 (goods ≤ ½ materials) rule.
* `IndustrySystem`: apply 2:1 transforms up to **site capacities** using **Labour + Power** as throughput caps; then pay unit build queues (regiments/ships).
* `WorkforceSystem`: resolve migration caps, training costs (paper + cash), daily food preferences, sickness/starvation, canned food fallback.
* `TradePhaseSystem`:

    * **Bids/Offers** placement → produce the **offer sequence** by commodity order.
    * Enforce **merchant marine holds** and **presence** rule for market visibility; compute **overseas profits** from Minor sales where you own land.
    * Update **Deal Book** / credit & forced sale checks.
* `DiplomacySystem`: store relation scores, subsidies/boycotts state, overture queue, council clock & votes.
* `ArmyRailMoveSystem`: allow **Train** moves within borders up to capacity; attacks require adjacency + war. Entrench defenders; apply fort bonuses.
* `NavalControlSystem`: compute each sea zone’s **command**; if enemy has **undisputed command**, break adjacent port connections; handle **intercepts** and **escorts**; repair in port.
* `TechSystem`: publish available inventions by era/year; allow **cash investment** to unlock; trigger replacements/unlocks in units and civilians.

---

## 10) UI parity (thin state over data)

* **Map**: province selector, garrison book (view regiments; toggle Available/Defending; upgrade when arrow appears; rename).
* **Transport**: 2‑page **book** with commodity list, sliders, capacity bar; greyed items are unavailable.
* **Industry**: clickable buildings = production dialogs; **Warehouse** is read‑only inventory pane. **Trade School**, **Capitol** (migration), **Shipyard**, **Railyard**, **Power Plant** are special buildings with their dialogs.
* **Trade**: **Bids & Offers** table + **Trade Book** tabs; **Deal Book** after resolution.
* **Diplomacy**: info, **Treaty Map**, **Relations** heat, **Trade Policies** (boycott/subsidize), **Council** tab.

---

## 11) Edge rules worth getting exactly right

* **Depot/Port gathering radius** is **own tile + 6 neighbors**, unique gatherer per tile; spacing matters.
* **Rail move cost** is tied to **armaments points** of a unit (5 capacity per point). Keep this integer‑based to make movement limits legible.
* **Food preference rotation** in groups of four workers (grain / fruit / livestock|fish / repeat); canned food is **universal**, stops sickness.
* **Militia** are immobile, free, auto‑upgrade, and always defend hometown only.
* **Naval “range wins”**: late‑era ships with longer range massacre earlier classes. Don’t over‑simulate; weight range heavily.
* **Ports & sea control**: if the enemy has **undisputed** command in a sea zone, your adjacent ports **disconnect** that turn.

---

## 12) Balance constants (from the manual; keep unless you want to redo economy)

* Per‑tile outputs by **ResourceDev** (see §2).
* **2→1** ratios in every economy; **steel = 1 iron + 1 coal**; **armaments** consume **steel**; **canned food** recipe ratio.
* **Factory/mill** cap build cost: **1 lumber + 1 steel** each; mills start at **2**, factories at **1**.
* **Fort** mitigation: **20% entrenchment + 10% per fort level (×3)** → **50%** at max.
* **Recruit cap**: ⌊provinces/4⌋ then ⌊provinces/3⌋ after upgrade.

---

## 13) Implementation notes / traps

* **Don’t tie army rail moves to Transport sliders**. Use **total capacity** as a separate check so players don’t micro sliders before every offensive (matches the manual’s intent).
* **Network recompute** only when rails/ports/ownership/sea control change; cache **Depot catchments** (tile+neighbors) by ID.
* Keep **commodities** as a single enum with **category tags** (`Resource/Material/Good`); the 2:1 transforms then become a pure data table.
* **Overseas profits**: settle them **after trade** into Treasury (do not deliver the physical goods to your Warehouse unless you actually bought them).
* **Council timing**: run a decade clock and a “who gets nominated” pass keyed to composite power (industry/diplomacy/military). UI just shows the vote map.

---

## 14) Minimal feature checklist to hit original feel

* [ ] Prospector → mineral discovery gates (incl. Oil after tech).
* [ ] Depot/Port catchments + connection rules + blockade logic.
* [ ] Transport sliders + rail capacity + army rail move cap (5×).
* [ ] 2:1 industry pipelines + labour + power + food preference/sickness.
* [ ] Trade Book / Deal Book flow + merchant marine limits + offers in commodity order.
* [ ] Diplomacy: embassies, overtures, subsidies/boycotts, council.
* [ ] Regiments: upgrades by tech table, militia immobility, forts/entrenchments.

Here’s the **very‑general, top‑down** take on *Imperialism* (1997) mechanics you’ll want in your head before re‑implementing anything.

---

## Board & screens

* **Hex map**: the world is a hex‑tiled grid (pointy‑top look); sea zones make the hex tiling obvious in political view. Provinces overlay those tiles.
* **Core UI modes**: one central **Terrain/Map** screen plus four orders screens: **Transport**, **Industry**, **Bid & Offers (trade)**, **Diplomacy**. You bounce through these each turn; everything commits on End Turn.

## Turn structure

* **Simultaneous resolution**: players queue orders; when all hit **End Turn**, the engine resolves in a fixed phase order: diplomacy → trade → industry/production → battles → trade cancellations from blockades → deliveries to warehouse. **One turn = three months.**
* **Unit cycle**: the game surfaces available units one by one for you to issue orders; nothing is final until End Turn.

## Geography & ownership

* **Province is the atomic military space**: armies/garrisons sit in provinces; **you fight and capture whole provinces, never individual tiles**. Provinces can’t be split between owners at the end of a turn.
* **Province contents**: exactly one town (or a capital) per province. Capitals yield base resources from their tile and are special for connectivity.
* **Sea**: oceans are split into **sea zones**, not owned like provinces, but **undisputed presence** in a zone lets a fleet cut port connections (blockades).

## Units & movement (per‑turn basics)

* **Civilians (engineer, prospector, miner/driller, farmer, forester, rancher, developer)** act on **tiles**. They can **move any distance per turn** (subject to borders/embassies) and typically **perform one job that consumes the turn** (e.g., build a rail segment, improve a tile).
* **Armies (regiments)** live in a province’s garrison. In a single turn you can:

    * **March** to an adjacent friendly province.
    * **Rail‑move** farther within friendly land if you have spare **transport capacity** (rail tonnage).
    * **Attack** only if adjacent (or conduct a **naval landing** via fleets).
      These are picked via map cursors (marching soldier / train / crossed swords).
* **Fleets** move by sea zones; speed stat = zones per turn; they can escort, intercept, blockade; experience and range matter in autoresolved naval combat.

## Economy & transport (why rails matter)

* **Tiles produce resources** (grain, timber, cotton, wool, iron, coal, gold, gems, oil) once developed; **prospector must discover minerals/oil first**.
* **Transport network**: resources only reach industry if their **tile is connected** via **rail → depot → capital or port** (ports/river/coast rules apply). Engineers lay rail, build depots/ports; capacity (from railyard/merchant marine) limits how much you can move per turn.
* **Industry/Trade**: transported inputs hit the **warehouse** next turn, then get turned into higher‑tier goods/units on the **Industry** screen; world market trading is set on **Bid & Offers**. All of this executes at End Turn.

## War, conquest, colonies (the high notes)

* **Province‑by‑province conquest**: you take provinces whole; taking a country’s **capital** can throw its remaining provinces into anarchy until taken.
* **Minor nations** can be **colonized** by diplomacy/economy; once a colony, you still develop tiles but its outputs flow via market rules, not straight into your rail net.

That’s the 10,000‑meter view: **hex tiles for economy, provinces for war, one meaningful order per unit per turn, everything resolves on End Turn**. If you want, I can translate this into a Bevy/Rust ECS sketch (components/systems for hex grid, province layer, unit cycle, and the End‑Turn pipeline).