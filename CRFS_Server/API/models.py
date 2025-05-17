from django.db import models


class User(models.Model):
    """A single user, identified by UUID."""

    uuid: models.Field = models.UUIDField(primary_key=True, editable=False)
    display_name: models.Field = models.CharField(max_length=256, null=True)
    last_seen: models.Field = models.DateTimeField()

    def __str__(self) -> str:
        if self.display_name:
            return f"User '{self.display_name}' Object"
        else:
            return "Unnamed User Object"


class FileSystem(models.Model):
    """A FileSystem, owned by a user.

    Represents a file tree which is synchronised across a number of replicas.
    """

    uuid: models.Field = models.UUIDField(primary_key=True, editable=False)
    user: models.Field = models.ForeignKey(User, on_delete=models.CASCADE, null=False, blank=False)
    display_name: models.Field = models.CharField(max_length=256, null=True)
    last_seen: models.Field = models.DateTimeField()
    opts: models.Field = models.TextField(default="")

    def __str__(self) -> str:
        if self.user.display_name:
            owner = "Unnamed User"
        else:
            owner = self.user.display_name

        if self.display_name:
            return f"FS {self.display_name}, of {owner}"
        else:
            return f"Unnamed FS, of {owner}"


class Replica(models.Model):
    """Tracks a replica on a user's device.

    A replica is an instance/copy of the files in a FileSystem on a given device.
    This system is designed to synchronise and resolve file changes across replicas.
    """

    uuid: models.Field = models.UUIDField(primary_key=True, editable=False)
    filesystem: models.Field = models.ForeignKey(FileSystem, on_delete=models.CASCADE, null=False, blank=False)
    last_seen: models.Field = models.DateTimeField()


class Operation(models.Model):
    """A single operation in a FS's history.

    Will always refer to a file existing at BASE_DIR/operations/<fs>/<hash>
    """

    filesystem: models.Field = models.ForeignKey(FileSystem, on_delete=models.CASCADE, null=False, blank=False)
    hash: models.Field = models.CharField(max_length=64, null=False, blank=False, primary_key=True)
