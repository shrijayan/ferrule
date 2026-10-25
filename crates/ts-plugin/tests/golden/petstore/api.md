# Shared

Types:

- <code><a href="./src/resources/shared.ts">Owner</a></code>

# Pets

Types:

- <code><a href="./src/resources/pets/pets.ts">Pet</a></code>
- <code><a href="./src/resources/pets/pets.ts">PetRetrieveResponse</a></code>
- <code><a href="./src/resources/pets/pets.ts">PetListResponse</a></code>

Methods:

- <code title="post /pets">client.pets.<a href="./src/resources/pets/pets.ts">create</a>({ ...params }) -> Pet</code>
- <code title="get /pets/{pet_id}">client.pets.<a href="./src/resources/pets/pets.ts">retrieve</a>(petID) -> PetRetrieveResponse</code>
- <code title="get /pets">client.pets.<a href="./src/resources/pets/pets.ts">list</a>({ ...params }) -> PetListResponse</code>

## Photos

Methods:

- <code title="delete /pets/{pet_id}/photos/{photo_id}">client.pets.photos.<a href="./src/resources/pets/photos.ts">delete</a>(photoID, { ...params }) -> void</code>
