use autorebase::{autorebase, test_utils::*};

#[test]
fn test_build_repo() {
    let root =
        commit("Hello")
        .write("a.txt", "hello")
        .child(
            commit("World")
            .write("a.txt", "world")
            .branch("master")
        );

    let repo = build_repo(&root, Some("master"));

    let repo_dir = repo.path();

    print_log(&repo_dir);

    let graph = get_repo_graph(&repo_dir).expect("error getting repo graph");
    dbg!(graph);
}


macro_rules! commit_info_graph {
    (
        $(
            $hash:literal : CommitInfo {
                $($field_name:ident : $field_value:tt),*
                $(,)?
            }
        ),*
        $(,)?
    ) => {{
        let mut graph: ::std::collections::BTreeMap<String, CommitInfo> = ::std::collections::BTreeMap::new();
        $(

            graph.insert($hash.to_string(), {
                #[allow(unused_mut)]
                let mut info: CommitInfo = Default::default();
                $(
                    commit_info_graph!(@set_field info, $field_name, $field_value);
                )*
                info
            });
        )*
        graph
    }};

    (@set_field $object:ident, parents, [ $($hash:expr),* $(,)? ]) => {
        $object.parents = vec![$($hash.to_string(), )*];
    };

    (@set_field $object:ident, refs, { $($hash:literal),* $(,)? }) => {
        $object.refs = {
            #[allow(unused_mut)]
            let mut r = ::std::collections::BTreeSet::new();
            $(
                r.insert($hash.to_string());
            )*
            r
        };
    };
}



#[test]
fn test_basic_autorebase() {
    let root =
        commit("First")
        .write("a.txt", "hello")
        .child(
            commit("Second")
            .write("a.txt", "world")
            .branch("master")
        )
        .child(
            commit("WIP")
            .write("b.txt", "foo")
            .branch("wip")
        );

    let repo = build_repo(&root, Some("master"));

    let repo_dir = repo.path();

    print_log(&repo_dir);

    let graph = get_repo_graph(&repo_dir).expect("error getting repo graph");
    dbg!(graph);

    autorebase(repo_dir, "master").expect("error autorebasing");

    print_log(&repo_dir);

    let graph = get_repo_graph(&repo_dir).expect("error getting repo graph");

    let expected_graph = commit_info_graph!(
        "04417144022049d97bc19e759d1955958e21f339": CommitInfo {
            parents: [
                "a6de41485a5af44adc18b599a63840c367043e39",
            ],
            refs: {
                "wip",
            },
        },
        "a6de41485a5af44adc18b599a63840c367043e39": CommitInfo {
            parents: [
                "d3591307bd5590f14ae24d03ab41121ab94e2a90",
            ],
            refs: {
                "master",
            },
        },
        "d3591307bd5590f14ae24d03ab41121ab94e2a90": CommitInfo {
            parents: [],
            refs: {
                "",
            },
        },
    );
    assert_eq!(graph, expected_graph);
}
