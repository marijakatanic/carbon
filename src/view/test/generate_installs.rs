use rand::seq::IteratorRandom;

pub(crate) async fn generate_installs(
    genesis_height: usize,
    max_height: usize,
    unskippable_count: usize,
    installable_count: usize,
) -> Vec<(usize, usize, Vec<usize>)> {
    assert!(
        installable_count <= unskippable_count
            && unskippable_count <= max_height - genesis_height - 1
    );

    let mut rng = rand::thread_rng();

    let mut unskippable = (genesis_height + 1..=max_height - 2)
        .choose_multiple(&mut rng, unskippable_count)
        .into_iter()
        .enumerate()
        .map(|(index, height)| (height, index < installable_count))
        .collect::<Vec<_>>();

    unskippable.sort_by_key(|(height, _)| *height);
    unskippable.push((max_height - 1, false));

    let mut installs = Vec::new();
    let mut current_unskippable = genesis_height;

    // Generate installs between all unskippable views (this includes the last view)

    for (parity, (next_unskippable, is_installable)) in unskippable.into_iter().enumerate() {
        let tail = if is_installable {
            vec![]
        } else {
            vec![next_unskippable + 1]
        };

        let mut new_installs = Vec::new();
        new_installs.push((current_unskippable, next_unskippable, tail));

        let mut must_include = Vec::new();

        for to_include in (current_unskippable + 1..next_unskippable)
            .choose_multiple(&mut rng, next_unskippable - current_unskippable - 1)
        {
            must_include.push(to_include);
            must_include.sort();

            let mut hops = must_include
                .clone()
                .into_iter()
                .chain(vec![next_unskippable].into_iter());

            new_installs.push((
                current_unskippable,
                hops.next().unwrap(),
                hops.collect::<Vec<_>>(),
            ));
        }

        current_unskippable = next_unskippable;

        if parity % 2 == 0 {
            installs.extend(new_installs.into_iter());
        } else {
            installs.extend(new_installs.into_iter().rev());
        };
    }

    installs
}

pub(crate) fn last_installable<I>(
    genesis_height: usize,
    max_height: usize,
    tailless: I,
) -> Vec<usize>
where
    I: IntoIterator<Item = usize>,
{
    let mut last_installable = Vec::new();
    let mut current_height = genesis_height;

    for next_height in tailless.into_iter() {
        while last_installable.len() < next_height {
            last_installable.push(current_height);
        }

        current_height = next_height;
    }

    while last_installable.len() < max_height {
        last_installable.push(current_height);
    }

    last_installable
}
