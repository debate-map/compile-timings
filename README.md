A repo for holding outputs of ```cargo build --timings``` performed by Github Actions in ```debate-map/app``` repo.

### Overall workflow:
- You make any **push**/**pull-req** operation on `debate-map/app` repo (see the github action workflow file in `debate-map/app` repo)
- The github actions workflow then runs `cargo build --timings` and pushes the html file generated into this repository in the `timings/raw_html` directory
- After receiving this push event, the github action workflow in **this repository** is triggered to go through all the files in the `timings/raw_html` directory
 and extract out all the timestamp metadata and builds the one that are not in the `timings/tracker.json` and their JSON files are populated in `timings/build_metadatas` and `timings/build_units`
![image](https://github.com/user-attachments/assets/433afa8a-5bcd-4bc8-a754-50dc04a59079)
