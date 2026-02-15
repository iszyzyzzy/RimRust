use super::base_list::*;
use crate::types::*;

use core::panic;
use serde::{Deserialize, Serialize};
use tracing_subscriber::field::debug;
use std::{
    cmp::Reverse,
    collections::{BinaryHeap, HashMap},
    vec,
};
use tracing::{debug, info, warn, trace, instrument};

// TODO 懒得改了先hashset转vec应付一下

impl ModInner {
    fn get_version_matched_deps(&self, game_version: Version) -> Option<Vec<ModDependency>> {
        return self.dependencies.get(&game_version).map(|deps| deps.iter().cloned().collect());
    }
    fn get_version_match_orders(&self, game_version: Version) -> Option<Vec<ModOrder>> {
        return self.load_order.get(&game_version).map(|orders| orders.iter().cloned().collect());
    }
    fn get_version_match_incompatible(&self, game_version: Version) -> Option<Vec<PackageId>> {
        return self.incompatible_with.get(&game_version).map(|incompatible| {
            incompatible.iter().cloned().collect()
        });
    }
}

#[derive(Clone, Debug)]
struct ModNode {
    id: Id,
    name: String,
    package_id: PackageId,
    edges: Vec<ModOrder>,
}

impl ModNode {
    fn to_order_node(&self) -> OrderNode {
        OrderNode::new(self.id.clone(), self.name.clone())
    }
}

#[derive(Eq, Clone, Debug)]
struct OrderNode {
    id: Id,
    name: String,
}
impl OrderNode {
    fn new(id: Id, name: String) -> Self {
        Self { id, name }
    }
}

impl Ord for OrderNode {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.name.cmp(&other.name)
    }
}

impl PartialOrd for OrderNode {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for OrderNode {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

#[derive(Clone, Debug)]
enum OrderType {
    UserDefined(ModOrder),
    ModDefined(ModOrder),
    Community(ModOrder),
}
impl OrderType {
    fn get_priority(&self) -> u8 {
        match self {
            OrderType::UserDefined(_) => 3,
            OrderType::ModDefined(_) => 2,
            OrderType::Community(_) => 0,
        }
    }
    fn get_inner(&self) -> ModOrder {
        match self {
            OrderType::UserDefined(order) => order.clone(),
            OrderType::ModDefined(order) => order.clone(),
            OrderType::Community(order) => order.clone(),
        }
    }
}

impl BaseList {
    async fn build_enabled_package_id_to_mod(
        &self,
        game_version: Version,
        warning: &mut Vec<ModSortWarning>,
    ) -> HashMap<PackageId, Id> {
        let mut package_id_to_mod: HashMap<PackageId, Id> = HashMap::new();
        let trans = self.translation_mod_data.lock(Priority::HIGH).await;
        let trans_cache = trans.get_all_translation_pack();
        drop(trans);
        for item in self.mods_map.iter() {
            let mod_data = item.lock().await;
            if !mod_data.enabled {
                continue;
            }
            // 扫描重复的package_id
            if let Some(exist) = package_id_to_mod.get(&mod_data.package_id) {
                warning.push(ModSortWarning::DuplicatePackageId(
                    mod_data.package_id.clone(),
                ));
                let exist_ = self.mods_map.get(exist).unwrap();
                let exist_ = exist_.lock().await;
                if !exist_.supported_version.contains(&game_version)
                    && mod_data.supported_version.contains(&game_version)
                {
                    package_id_to_mod.insert(mod_data.package_id.clone(), mod_data.id.clone());
                } else if exist_.supported_version.contains(&game_version)
                    && !mod_data.supported_version.contains(&game_version)
                {
                    // keep exist
                } else {
                    // 都支持，按名字字母序
                    if mod_data.name < exist_.name {
                        package_id_to_mod.insert(mod_data.package_id.clone(), mod_data.id.clone());
                    }
                }
            } else {
                package_id_to_mod.insert(mod_data.package_id.clone(), mod_data.id.clone());
            }
            // 扫描不支持的版本
            if !mod_data.supported_version.contains(&game_version) {
                // 看一眼是不是汉化包，汉化包是无所谓版本的
                if !trans_cache.contains(&mod_data.id) {
                    warning.push(ModSortWarning::VersionMismatch(
                        mod_data.id.clone(),
                        game_version.clone(),
                        mod_data.supported_version.iter().map(|(k,_)| k.clone()).collect(),
                    ));
                }
            }
        }
        package_id_to_mod
    }

    async fn build_mod_node(
        &self,
        mod_: &ModInner,
        package_id_to_mod: &HashMap<PackageId, Id>,
        game_version: Version,
        error: &mut Vec<ModSortError>,
        priority: Option<Priority>,
    ) -> ModNode {
        let mut edges_by_target: HashMap<Id, Vec<OrderType>> = HashMap::new();

        if let Some(deps) = &mod_.get_version_matched_deps(game_version.clone()) {
            for dep in deps {
                if package_id_to_mod.contains_key(&dep.package_id) {
                    edges_by_target
                        .entry(package_id_to_mod.get(&dep.package_id).unwrap().clone())
                        .or_insert(vec![])
                        .push(OrderType::ModDefined(ModOrder::After(
                            dep.package_id.clone(),
                        )));
                } else if !dep.optional {
                    error.push(ModSortError::MissingDependency(
                        mod_.id.clone(),
                        dep.package_id.clone(),
                        dep.display_name.clone(),
                    ));
                }
            }
        }

        if let Some(user_order) = self
            .user_custom_mods_order
            .get(&mod_.package_id)
        {
            for order in user_order.value() {
                match order {
                    ModOrder::First() | ModOrder::Last() => {
                        edges_by_target
                            .entry(Id::enpty())
                            .or_insert(vec![])
                            .push(OrderType::UserDefined(order.clone()));
                    }
                    ModOrder::After(package_id) | ModOrder::Before(package_id) => {
                        if package_id_to_mod.contains_key(package_id) {
                            edges_by_target
                                .entry(package_id_to_mod.get(package_id).unwrap().clone())
                                .or_insert(vec![])
                                .push(OrderType::UserDefined(order.clone()));
                        }
                    }
                }
            }
        }

        if let Some(mod_order) = &mod_.get_version_match_orders(game_version.clone()) {
            for order in mod_order {
                match order {
                    ModOrder::After(package_id) | ModOrder::Before(package_id) => {
                        if package_id_to_mod.contains_key(package_id) {
                            edges_by_target
                                .entry(package_id_to_mod.get(package_id).unwrap().clone())
                                .or_insert(vec![])
                                .push(OrderType::ModDefined(order.clone()));
                        }
                    }
                    _ => {
                        // 根据wiki，About.xml里没法定义First和Last
                        warn!("ModOrder::First() and ModOrder::Last() should not be defined in About.xml");
                    }
                }
            }
        }
        if let Some(community_order) = self
            .community_data
            .get_community_rules()
            .lock(priority)
            .await
            .get(&mod_.package_id)
        {
            for order in community_order {
                match order {
                    ModOrder::First() | ModOrder::Last() => {
                        edges_by_target
                            .entry(Id::enpty())
                            .or_insert(vec![])
                            .push(OrderType::Community(order.clone()));
                    }
                    ModOrder::After(package_id) | ModOrder::Before(package_id) => {
                        if package_id_to_mod.contains_key(package_id) {
                            edges_by_target
                                .entry(package_id_to_mod.get(package_id).unwrap().clone())
                                .or_insert(vec![])
                                .push(OrderType::Community(order.clone()));
                        }
                    }
                }
            }
        }
        let debug_data = self.community_data.get_community_rules().lock(priority).await.get(&mod_.package_id).cloned();
        debug!(mod_id = ?mod_.id, data = ?debug_data, "社区数据");
        //debug!(mod_id = ?mod_.id, edges_by_target = ?edges_by_target, "构建ModNode的边");

        let mut final_edges: Vec<ModOrder> = vec![];
        for (_, mut edges) in edges_by_target {
            edges.sort_by(|a, b| a.get_priority().cmp(&b.get_priority()));
            if let Some(first) = edges.first() {
                final_edges.push(first.get_inner());
            }
        }

        // 只提错误。不影响排序
        if let Some(incompatibles) = &mod_.get_version_match_incompatible(game_version.clone()) {
            for incompatible in incompatibles {
                if let Some(incompatible_id) = package_id_to_mod.get(incompatible) {
                    error.push(ModSortError::IncompatibleMods(
                        mod_.id.clone(),
                        incompatible_id.clone(),
                    ));
                }
            }
        }

        let res = ModNode {
            id: mod_.id.clone(),
            name: mod_.name.clone(),
            package_id: mod_.package_id.clone(),
            edges: final_edges,
        };
        debug!(mod_id = ?mod_.id, node = ?res, "构建ModNode");
        res
    }

    async fn build_mod_graph(
        &self,
        game_version: Version,
        package_id_to_mod: &HashMap<PackageId, Id>,
        error: &mut Vec<ModSortError>,
        priority: Option<Priority>,
    ) -> HashMap<Id, ModNode> {
        let mut graph: HashMap<Id, ModNode> = HashMap::new();

        for mod_ in self.mods_map.iter() {
            let mod_data = mod_.lock().await;
            if !mod_data.enabled {
                continue;
            }
            let node = self
                .build_mod_node(
                    &mod_data,
                    &package_id_to_mod,
                    game_version.clone(),
                    error,
                    priority,
                )
                .await;
            graph.insert(mod_data.id.clone(), node);
        }

        // 规范化边的方向，全部处理成Before
        // a --> b; c <-- a => {a:[b,c]}
        // first 和 last 暂时不处理, 原样保留
        let mut normalized_edges: HashMap<Id, Vec<PackageId>> = HashMap::new();
        let mut firsts: Vec<Id> = vec![];
        let mut lasts: Vec<Id> = vec![];

        for (id, node) in &graph {
            for edge in &node.edges {
                match edge {
                    ModOrder::After(package_id) => {
                        normalized_edges
                            .entry(package_id_to_mod.get(package_id).unwrap().clone())
                            .or_insert(vec![])
                            .push(self.mods_map.get(id).unwrap().lock().await.package_id.clone());
                    }
                    ModOrder::Before(package_id) => {
                        normalized_edges
                            .entry(id.clone())
                            .or_insert(vec![])
                            .push(package_id.clone());
                    }
                    ModOrder::First() => {
                        firsts.push(id.clone());
                        /*                         normalized_edges.entry(id.clone()).or_insert(vec![]).extend(
                            futures::stream::iter(graph.keys().cloned()).then(|id| async move {
                                self.mods_map
                                    .get(&id)
                                    .unwrap()
                                    .lock()
                                    .await
                                    .package_id
                                    .clone()
                            }).collect::<Vec<PackageId>>().await
                        ); */
                    }
                    ModOrder::Last() => {
                        lasts.push(id.clone());
                        /*                         for (key, node) in &graph {
                            normalized_edges
                                .entry(key.clone())
                                .or_insert(vec![])
                                .push(node.package_id.clone());
                        } */
                    }
                }
            }
        }

        // 用规范化的边重建图
        let mut normalized_graph: HashMap<Id, ModNode> = HashMap::new();
        for (id, node) in &graph {
            let mut new_edges: Vec<ModOrder> = vec![];
            if let Some(edges) = normalized_edges.get(&id.clone()) {
                new_edges = edges
                    .iter()
                    .map(|id| ModOrder::Before(id.clone()))
                    .collect();
            }
            normalized_graph.insert(
                id.clone(),
                ModNode {
                    id: node.id.clone(),
                    name: node.name.clone(),
                    package_id: node.package_id.clone(),
                    edges: new_edges,
                },
            );
        }
        for id in firsts {
            normalized_graph
                .entry(id)
                .and_modify(|node| node.edges.push(ModOrder::First()));
        }
        for id in lasts {
            normalized_graph
                .entry(id)
                .and_modify(|node| node.edges.push(ModOrder::Last()));
        }

        normalized_graph
    }

    #[instrument(skip(self, game_version, priority))]
    pub async fn sort(
        &self,
        game_version: Version,
        priority: Option<Priority>,
    ) -> Result<SortResult, String> {
        info!("排序ing");
        let mut result = SortResult {
            list: vec![],
            error: vec![],
            warning: vec![],
            info: vec![],
        };
        let package_id_to_mod = self
            .build_enabled_package_id_to_mod(game_version.clone(), &mut result.warning)
            .await;
        info!("构建package_id_to_mod");
        debug!(map = ?package_id_to_mod, res = ?result);

        let graph = self
            .build_mod_graph(
                game_version.clone(),
                &package_id_to_mod,
                &mut result.error,
                priority,
            )
            .await;
        info!("构建mod_graph");
        debug!(map = ?graph, res = ?result);

        // 拓扑排序 Kahn算法 再按字母顺序
        // 分三层
        // 先把所有first和预定义列表里的以及他们的依赖找出来放在第一层
        // 然后是所有的last和定义列表以及依赖他们的放在第三层
        // 其余放在第二层
        // 最后打断层之间的依赖
        // 这个写法是从rimsort源码里看来的
        // 最主要的问题是很多mod没有显式的写明loadAfter core，我又不太好给他加

        let mut first_graph: HashMap<Id, ModNode> = HashMap::new();
        let mut last_graph: HashMap<Id, ModNode> = HashMap::new();
        let mut middle_graph = graph.clone(); // Start with all nodes in middle

        let mut first_mod_ids: Vec<Id> = vec![];
        let mut last_mod_ids: Vec<Id> = vec![];

        for (id, node) in &graph {
            if node.edges.contains(&ModOrder::First()) || FIRST_MOD.contains(&node.package_id) {
                first_mod_ids.push(id.clone());
            } else if node.edges.contains(&ModOrder::Last()) || LAST_MOD.contains(&node.package_id)
            {
                last_mod_ids.push(id.clone());
            }
        }

        // Move first mods and their dependencies to first_graph
        let mut stack: Vec<Id> = first_mod_ids;
        while let Some(id) = stack.pop() {
            if let Some(node) = middle_graph.remove(&id) {
                first_graph.insert(id.clone(), node.clone());
                // Find dependencies of this node and add them to the stack to be processed
                for dep_id in self.get_deps_recursive(id, &graph, &package_id_to_mod) {
                    stack.push(dep_id);
                }
            }
        }

        // Move last mods and those that depend on them to last_graph
        let mut stack: Vec<Id> = last_mod_ids;
        while let Some(id) = stack.pop() {
            if let Some(node) = middle_graph.remove(&id) {
                last_graph.insert(id.clone(), node.clone());
                // Find mods that depend on this node (reverse dependencies)
                for rev_dep_id in self.get_reverse_deps_recursive(id, &graph).await {
                    stack.push(rev_dep_id);
                }
            }
        }
        info!("分层");
        debug!(first = ?first_graph, last = ?last_graph, mid = ?middle_graph);


        result
            .list
            .extend(self.topo_sort(first_graph, &package_id_to_mod, &mut result.error));
        result
            .list
            .extend(self.topo_sort(middle_graph, &package_id_to_mod, &mut result.error));
        result
            .list
            .extend(self.topo_sort(last_graph, &package_id_to_mod, &mut result.error));

        info!("排序完成");
        trace!(res = ?result);
        Ok(result)
    }

    /// 这个函数没有递归地找依赖!这和reverse不一样
    fn get_deps_recursive(
        &self,
        id: Id,
        graph: &HashMap<Id, ModNode>,
        package_id_to_mod: &HashMap<PackageId, Id>,
    ) -> Vec<Id> {
        let mut dependencies = vec![];
        let target_package_id = &graph.get(&id).unwrap().package_id;

        for (potential_dep_id, node) in graph {
            for edge in &node.edges {
                if let ModOrder::Before(pkg_id) = edge {
                    if pkg_id == target_package_id {
                        dependencies.push(potential_dep_id.clone());
                    }
                }
            }
        }
        dependencies
    }
    async fn get_reverse_deps_recursive(
        &self,
        id: Id,
        graph: &HashMap<Id, ModNode>,
    ) -> Vec<Id> {
        let mut result = vec![];
        let mut stack = vec![id];
        while let Some(current_id) = stack.pop() {
            result.push(current_id.clone());
            for (id, node) in graph {
                if node.edges.contains(&ModOrder::Before(
                    self.mods_map
                        .get(&current_id)
                        .unwrap()
                        .lock()
                        .await
                        .package_id
                        .clone(),
                )) {
                    stack.push(id.clone());
                }
            }
        }
        result
    }
    fn topo_sort(
        &self,
        graph: HashMap<Id, ModNode>,
        package_id_to_mod: &HashMap<PackageId, Id>,
        error: &mut Vec<ModSortError>,
    ) -> Vec<Id> {
        let mut in_degree: HashMap<Id, usize> = HashMap::new();
        for (id, _) in &graph {
            in_degree.insert(id.clone(), 0);
        }
        for (_, node) in &graph {
            for edge in &node.edges {
                match edge {
                    ModOrder::Before(target_package_id) => {
                        if let Some(degree) = in_degree
                            .get_mut(package_id_to_mod.get(target_package_id).unwrap()) {
                            *degree += 1;
                        }
                    }
                    _ => panic!("Unexpected ModOrder in graph"),
                }
            }
        }

        let mut queue: BinaryHeap<Reverse<OrderNode>> = BinaryHeap::new();

        for (id, degree) in &in_degree {
            if *degree == 0 {
                queue.push(Reverse(graph.get(id).unwrap().to_order_node()));
            }
        }

        let mut result = vec![];

        while let Some(Reverse(current_id)) = queue.pop() {
            result.push(current_id.id.clone());

            let mut next_nodes = Vec::new();
            if let Some(node) = graph.get(&current_id.id) {
                for target in &node.edges {
                    match target {
                        ModOrder::Before(target_package_id) => {
                            let target_id = package_id_to_mod.get(target_package_id).unwrap();
                            if let Some(degree) = in_degree.get_mut(target_id) {
                                *degree -= 1;
                                if *degree == 0 {
                                    next_nodes.push(graph.get(target_id).unwrap().to_order_node());
                                }
                            }
                        }
                        _ => panic!("Unexpected ModOrder in graph"),
                    }
                }
            }

            for node in next_nodes {
                queue.push(std::cmp::Reverse(node));
            }
        }

        // 检查是否有环
        if result.len() != graph.len() {
            error.push(ModSortError::CircularDependency(
                graph.keys().cloned().collect(),
            ));
            // 塞到result最后
            for (id, _) in &graph {
                result.push(id.clone());
            }
        }

        result
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SortResult {
    pub list: Vec<Id>,
    pub error: Vec<ModSortError>,     // (package_ids, error_message)
    pub warning: Vec<ModSortWarning>, // (package_ids, warning_message)
    pub info: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ModSortError {
    CircularDependency(Vec<Id>),
    IncompatibleMods(Id, Id),
    MissingDependency(Id, PackageId, Option<String>), // (mod_id, package_id, name)
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ModSortWarning {
    ConflictingOrders(Id, Id),
    DuplicatePackageId(PackageId),
    VersionMismatch(Id, Version, Vec<Version>), // (mod_id, game_version, supported_versions)
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ModSortInfo {
    OptionalDependencyMissing(Id, PackageId),
}
