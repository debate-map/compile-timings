A repo for holding outputs of ```cargo build --timings``` performed by Github Actions in ```debate-map/app``` repo.

### Workflow:
- You make any push/pull req operation(see the github action workflow file in `debate-map/app` repo) on `debate-map/app` repo
- The workflow runs `cargo build --timings` and pushes the html file generated into this repository i.e `debate-map/compile-timings`
- After receiving this push event, the workflow runs (this is a TODO right now)
![image](https://github.com/user-attachments/assets/433afa8a-5bcd-4bc8-a754-50dc04a59079)
