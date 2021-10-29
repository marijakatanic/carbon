use rand::seq::IteratorRandom;

pub(crate) async fn generate_installs(
    genesis_height: usize,
    max_height: usize,
    unskippable_count: usize,
    installable_count: usize,
) -> Vec<(usize, usize, Vec<usize>)> {
    assert!(installable_count <= unskippable_count && unskippable_count <= max_height - 1);

    let mut rng = rand::thread_rng();

    let mut unskippable = (genesis_height + 1..=max_height - 2)
        .choose_multiple(&mut rng, unskippable_count)
        .into_iter()
        .enumerate()
        .map(|(i, height)| (height, i < installable_count))
        .collect::<Vec<_>>();

    unskippable.sort_by_key(|(a, _)| *a);

    unskippable.push((max_height - 1, false));

    let mut installs = Vec::new();
    let mut current_unskippable = genesis_height;

    // Generate installs between all unskippable views (this includes the last view)

    for (next_unskippable, is_installable) in unskippable {
        let tail = if is_installable {
            vec![]
        } else {
            vec![next_unskippable + 1]
        };

        installs.push((current_unskippable, next_unskippable, tail));

        let mut must_include = Vec::new();

        for to_include in (current_unskippable + 1..next_unskippable)
            .choose_multiple(&mut rng, next_unskippable - current_unskippable - 1)
        {
            must_include.push(to_include);
            must_include.sort();

            let mut v = must_include
                .clone()
                .into_iter()
                .chain(vec![next_unskippable].into_iter());

            installs.push((
                current_unskippable,
                v.next().unwrap(),
                v.collect::<Vec<_>>(),
            ));
        }

        current_unskippable = next_unskippable;
    }

    installs
}
