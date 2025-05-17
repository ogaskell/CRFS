from django.contrib import admin

from .models import FileSystem, Operation, Replica, User

admin.site.register(FileSystem)
admin.site.register(Replica)
admin.site.register(User)
admin.site.register(Operation)
