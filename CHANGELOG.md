## Unreleased

- Fixed: Structs with generics are now supported (https://github.com/heroku-buildpacks/cache_diff/pull/12)
- Fixed: Use fully qulified path to `::std::vec::Vec` (https://github.com/heroku-buildpacks/cache_diff/pull/8)

## 1.1.0

- Add `#[cache_diff(custom = <function>)]` to containers (structs) to allow for customizing + deriving diffs. (https://github.com/heroku-buildpacks/cache_diff/pull/6)
- Add: Allow annotating ignored fields with `#[cache_diff(ignore = "<reason>")]`. Using `ignore = "custom"` requires the container (struct) to implement `custom = <function>`. (https://github.com/heroku-buildpacks/cache_diff/pull/6)

## 1.0.1

- Fix: Multiple `#[derive(CachDiff)]` calls in the same file now work (https://github.com/heroku-buildpacks/cache_diff/pull/4)

## 1.0.0

- Changed: Error when deriving CacheDiff when zero comparison fields are found. This can happen if the struct has no fields or if all fields are `ignore`-d (https://github.com/schneems/cache_diff/pull/4)

## 0.1.0

- Initial release
