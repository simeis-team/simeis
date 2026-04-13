import json

with open(sys.argv[1], "r") as f:
    allpaths = json.load(f)

swagger = {
    "openapi": 3.0.4,
    "info": {
        "title": "Simeis",
        "description": "Game using API", # TODO Get from comment in main.rs
        "version": "0.1.3", # TODO Get from Cargo.toml
    },
    "servers": {},
    "paths": {},
}

examples_data = {}
# TODO Add sections in the swagger
# https://swagger.io/docs/specification/v3_0/grouping-operations-with-tags/ 
# TODO Do the "new player" api first
for path, data in allpaths.items():
    # TODO Check if pattern requires a player id, ship id, station id
    #     If not ready yet, do others and then loop back to them
    # TODO Fill the swagger
    # TODO Call the API to generate an example
    pass

with open("swagger.json", "w") as f:
    json.dump(swagger, f, indent=2)

# TODO Generate a full HTML page with the whole swagger-ui + swagger.json data directly in it
# (No need to serve other web resources)
