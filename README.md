A repo for holding outputs of ```cargo build --timings``` performed by Github Actions in ```debate-map/app``` repo.

### Overall workflow:
- You make any **push**/**pull-req** operation on `debate-map/app` repo (see the github action workflow file in `debate-map/app` repo)
- The github actions workflow then runs `cargo build --timings` and pushes the html file generated into this repository in the `timings/raw_html` directory
- After receiving this push event, the github action workflow in **this repository** is triggered to go through all the files in the `timings/raw_html` directory
 and extract out all the timestamp metadata and builds the one that are not in the `timings/tracker.json` and their JSON files are populated in `timings/build_metadatas` and `timings/build_units`
![image](https://github.com/user-attachments/assets/433afa8a-5bcd-4bc8-a754-50dc04a59079)

### NOTES:

If we look at the raw html file of `cargo build --timings`, then we can see that `async-graphql` at
[line 190](https://github.com/debate-map/compile-timings/blob/5b1585495f384af7aa7e884b7b9b00eae58f2268/docs/timings/raw_html/cargo-timing-20241006T165204Z.html#L190) and
[ line 198](https://github.com/debate-map/compile-timings/blob/5b1585495f384af7aa7e884b7b9b00eae58f2268/docs/timings/raw_html/cargo-timing-20241006T165204Z.html#L198) on the same file, which basically
indicates "that version of async-graphql with the exact same features was repeated".
(btw, the file take here is just for reference, for what's to be concluded)

So, the question arises, why is it repeated

One way to look at this is using `cargo tree` on the `app-server`, if we look at `async-graphql` there, we can see that
one is compiled under `rust-macros(for the proc macros)`. So, the initial guess is that "compilation for proc macros are isolated and they aren't unified".
But it seems the answer is bit different if we look at
[this comment](https://github.com/rust-lang/cargo/issues/13321#issuecomment-1899332106)

So, maybe this could be one of the potential issue for bigger build time?

Also, one problem that this creates is for the statistics(where we compare build timing of two different timing data) i.e in the `compile-timing-viewer` we had initial assumption that each build unit name would be unique, which means for a specific version the build would occur once (not like the above case where `async-graphql` is repeated twice).
So, what we've done currently is, combined the time of those two builds to display as one(well this does hide the fact it was compiled twice, so to preserve that info what we can do is put something like **2x**, if we have two builds for same unit)
