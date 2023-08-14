use rand::random;

use hloo_core::{BitContainer, BitPermuter};
use hloo_macros::make_permutations;

#[test]
fn apply_works_correctly() {
    make_permutations!(struct_name = "Permutations", f = 64, r = 5, k = 2, w = 32);
    // 64 / 5 = 13, 13, 13, 13, 12
    let bits = Bits::new([
        0b1111111111111_1010101010101_000110,
        0b0110011_0000000000000_111100001111,
    ]);
    // combinations of the first two blocks:
    let mut expected = vec![
        //
        0b1111111111111_1010101010101_000000u32,
        0b1111111111111_0001100110011_000000u32,
        0b1111111111111_0000000000000_000000u32,
        0b1111111111111_111100001111_0000000u32,
        //
        0b1010101010101_0001100110011_000000u32,
        0b1010101010101_0000000000000_000000u32,
        0b1010101010101_111100001111_0000000u32,
        //
        0b0001100110011_0000000000000_000000u32,
        0b0001100110011_111100001111_0000000u32,
        //
        0b0000000000000_111100001111_0000000u32,
    ];
    assert_eq!(Permutations::get_all_variants().len(), 10);
    for (pi, perm) in Permutations::get_all_variants().iter().enumerate() {
        let res = perm.apply(&bits);
        let found = expected
            .iter()
            .enumerate()
            .find(|(_, mask)| res.data[0] & **mask == **mask);
        if let Some((i, _)) = found {
            expected.remove(i);
        } else {
            assert!(false, "permutation #{} produced unexpected result!", pi)
        }
    }
    assert!(expected.is_empty(), "not all patterns were matched!")
}

#[test]
fn apply_works_correctly_with_rest_ordering_preserved() {
    make_permutations!(struct_name = "Permutations", f = 64, r = 5, k = 2, w = 32);
    // 64 / 5 = 13, 13, 13, 13, 12
    let bits = Bits::new([
        0b1111111111111_1010101010101_000110,
        0b0110011_0000000000000_111100001111,
    ]);
    // combinations of the first two blocks:
    // (('1111111111111', 0), ('1010101010101', 1))
    // (('1111111111111', 0), ('0001100110011', 2))
    // (('1111111111111', 0), ('0000000000000', 3))
    // (('1111111111111', 0), ('111100001111', 4))
    // (('1010101010101', 1), ('0001100110011', 2))
    // (('1010101010101', 1), ('0000000000000', 3))
    // (('1010101010101', 1), ('111100001111', 4))
    // (('0001100110011', 2), ('0000000000000', 3))
    // (('0001100110011', 2), ('111100001111', 4))
    // (('0000000000000', 3), ('111100001111', 4))
    let mut expected = vec![
        //
        [
            0b1111111111111_1010101010101_000110u32,
            0b0110011_0000000000000_111100001111u32,
        ],
        [
            0b1111111111111_0001100110011_101010u32,
            0b1010101_0000000000000_111100001111u32,
        ],
        [
            0b1111111111111_0000000000000_101010u32,
            0b1010101_0001100110011_111100001111u32,
        ],
        [
            0b1111111111111_111100001111_1010101u32,
            0b010101_0001100110011_0000000000000u32,
        ],
        //
        [
            0b1010101010101_0001100110011_111111u32,
            0b1111111_0000000000000_111100001111u32,
        ],
        [
            0b1010101010101_0000000000000_111111u32,
            0b1111111_0001100110011_111100001111u32,
        ],
        [
            0b1010101010101_111100001111_1111111u32,
            0b111111_0001100110011_0000000000000u32,
        ],
        //
        [
            0b0001100110011_0000000000000_111111u32,
            0b1111111_1010101010101_111100001111u32,
        ],
        [
            0b0001100110011_111100001111_1111111u32,
            0b111111_1010101010101_0000000000000u32,
        ],
        //
        [
            0b0000000000000_111100001111_1111111u32,
            0b111111_1010101010101_0001100110011u32,
        ],
    ];
    assert_eq!(Permutations::get_all_variants().len(), 10);
    for (pi, perm) in Permutations::get_all_variants().iter().enumerate() {
        let res = perm.apply(&bits);
        let found = expected.iter().enumerate().find(|(_, val)| res.data == **val);
        if let Some((i, _)) = found {
            expected.remove(i);
        } else {
            assert!(false, "permutation #{} produced unexpected result!", pi)
        }
    }
    assert!(expected.is_empty(), "not all patterns were matched!")
}

#[test]
fn apply_then_revert_is_identity() {
    make_permutations!(struct_name = "Permutations", f = 64, r = 5, k = 2, w = 32);

    let bits = Bits::new(random());
    for (i, perm) in Permutations::get_all_variants().iter().enumerate() {
        let permuted = perm.apply(&bits);
        let reverted = perm.revert(&permuted);
        assert_eq!(bits, reverted, "permutation {}: failed apply-revert test!", i);
    }
}

#[test]
fn mask_works_correctly() {
    make_permutations!(struct_name = "Permutations", f = 64, r = 5, k = 2, w = 32);
    // 64 / 5 = 13, 13, 13, 13, 12
    let bits = Bits::new([
        0b1111111111111_1010101010101_000110,
        0b0110011_0000000000000_111100001111,
    ]);
    // combinations of the first two blocks:
    let mut expected = vec![
        //
        0b1111111111111_1010101010101_000000u32,
        0b1111111111111_1010101010101_000000u32,
        0b1111111111111_1010101010101_000000u32,
        0b1111111111111_101010101010_0000000u32,
        //
        0b1111111111111_1010101010101_000000u32,
        0b1111111111111_1010101010101_000000u32,
        0b1111111111111_101010101010_0000000u32,
        //
        0b1111111111111_1010101010101_000000u32,
        0b1111111111111_101010101010_0000000u32,
        //
        0b1111111111111_101010101010_0000000u32,
    ];

    for (pi, perm) in Permutations::get_all_variants().iter().enumerate() {
        let res = perm.mask(&bits);
        let found = expected.iter().enumerate().find(|(_, mask)| res.data[0] == **mask);
        if let Some((i, _)) = found {
            expected.remove(i);
        } else {
            assert!(
                false,
                "permutation #{} produced unexpected result! {}",
                pi,
                res.to_string()
            )
        }
    }
}

#[test]
fn iter_works_correctly() {
    make_permutations!(struct_name = "Permutations", f = 64, r = 5, k = 1, w = 32);
    let bits = Bits::new([
        0b10101010101010101010101010101010u32,
        0b10101010101010101010101010101010u32,
    ]);

    let res: Vec<_> = bits.iter().enumerate().collect();
    assert!(
        res.iter().all(|(i, v)| (i % 2 == 0 && *v) || (i % 2 != 0 && !*v)),
        "bits.iter does not produce expected result"
    );

    let reconstructed = Bits::from_iter(res.into_iter().map(|(_, v)| v));
    assert_eq!(reconstructed, bits, "bits.from_iter is unable to reconstruct bits");
}
