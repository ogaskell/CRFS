"""API Message handlers."""

import uuid
from collections.abc import Callable

from django.core.exceptions import ObjectDoesNotExist
from django.utils import timezone

from .models import FileSystem, User


class JSONMessageHandler:
    """Handler class fr all message-based requests.

    Takes a method to handle well-formed messages.
    Will check if message is well formed, if so uses the handler, else returns an error.
    Also forms the reply message with the appropriate fields.
    """

    def __init__(self, handler: Callable[[str, dict, str], tuple[int, dict]]):
        """Initialise handler class with payload handler method.

        Handler should have signature `handler(message_type: str, payload: dict, http_method: str)` and should return a tuple
        of (http_code: int, reply_payload: dict).
        """
        self.handler = handler

    def handle_message(self, request_data: dict, http_method: str) -> tuple[int, dict]:
        """Handle a request."""
        reply_payload = None

        try:
            id = request_data["transaction_id"]
        except KeyError:
            http_code = 400
            reply_payload = {"code": 8, "err_msg": "Missing mandatory field \"transaction_id\"."}

        try:
            message_type = request_data["type"]
        except KeyError:
            http_code = 400
            reply_payload = {"code": 8, "err_msg": "Missing message type."}

        try:
            payload = request_data["payload"]
        except KeyError:
            http_code = 400
            reply_payload = {"code": 8, "err_msg": "Missing message payload."}

        if reply_payload is None:
            http_code, reply_payload = self.handler(message_type, payload, http_method)

        return http_code, {
            "version": "1.0",
            "transaction_id": id,
            "reply": True,
            "message_type": message_type,
            "payload": reply_payload,
            "notifications": [],
        }


def ping_handler(message_type: str, payload: dict, http_method: str) -> tuple[int, dict]:
    """Handle `ping` messages."""
    return 200, {}


def register_user_handler(message_type: str, payload: dict, http_method: str) -> tuple[int, dict]:
    """Handle `register_user` messages."""
    if "user_uuid" in payload.keys():
        user_uuid = payload["user_uuid"]
    else:
        return (400, {"code": 8, "err_msg": "Missing field \"user_uuid\" required by type \"register_user\"."})

    if "display_name" in payload.keys():
        dispname = payload["display_name"]
    else:
        dispname = None

    try:
        user = User.objects.get(uuid=uuid.UUID(user_uuid))

        user.display_name = dispname
        user.last_seen = timezone.now()
    except ObjectDoesNotExist:
        user = User(
            uuid=uuid.UUID(user_uuid),
            display_name=dispname,
            last_seen=timezone.now()
        )

    user.save()

    return 200, {
        "user_uuid": str(user.uuid),
        "display_name": user.display_name,
    }


def check_user_handler(message_type: str, payload: dict, http_method: str) -> tuple[int, dict]:
    """Handle `check_user` messages."""
    if "user_uuid" in payload.keys():
        user_uuid = payload["user_uuid"]
    else:
        return (400, {"code": 8, "err_msg": f"Missing field \"user_uuid\" required by type \"{message_type}\"."})

    try:
        user = User.objects.get(uuid=uuid.UUID(user_uuid))
        user.last_seen = timezone.now()

        user.save()

        return 200, {
            "user_uuid": str(user.uuid),
            "display_name": user.display_name
        }
    except ObjectDoesNotExist:
        return 400, {
            "code": 3
        }


def register_filesystem_handler(message_type: str, payload: dict, http_method: str) -> tuple[int, dict]:
    """Handle `register_fs`."""
    if "user_uuid" in payload.keys():
        user_uuid = payload["user_uuid"]
    else:
        return (400, {"code": 8, "err_msg": "Missing field \"user_uuid\" required by type \"register_fs\"."})

    if "fs_uuid" in payload.keys():
        fs_uuid = payload["fs_uuid"]
    else:
        return (400, {"code": 8, "err_msg": "Missing field \"fs_uuid\" required by type \"register_fs\"."})

    if "display_name" in payload.keys():
        dispname = payload["display_name"]
    else:
        dispname = None

    if "fs_opts" in payload.keys():
        fs_opts_raw = payload["display_name"]
        if not isinstance(fs_opts_raw, list):
            return (400, {"code": 8, "err_msg": "Field \"fs_opts\" must be a list."})

        fs_opts = " ".join(fs_opts_raw)
    else:
        fs_opts = ""

    try:
        user = User.objects.get(uuid=uuid.UUID(user_uuid))
    except ObjectDoesNotExist:
        return (400, {"code": 3})

    try:
        fs = FileSystem.objects.get(uuid=uuid.UUID(fs_uuid))

        if fs.user != user:
            return (400, {"code": 9, "err_msg": "FileSystem with given UUID is already owned by another user."})

        fs.display_name = dispname
        fs.opts = fs_opts
        fs.last_seen = timezone.now()
    except ObjectDoesNotExist:
        fs = FileSystem(
            uuid=uuid.UUID(fs_uuid),
            user=user,
            display_name=dispname,
            last_seen=timezone.now(),
            opts=fs_opts,
        )

    fs.save()

    return 200, {
        "user_uuid": str(user.uuid),
        "fs_uuid": str(fs.uuid),
        "display_name": fs.display_name,
    }


PingHandler = JSONMessageHandler(ping_handler)
RegisterUserHandler = JSONMessageHandler(register_user_handler)
CheckUserHandler = JSONMessageHandler(check_user_handler)
RegisterFSHandler = JSONMessageHandler(register_filesystem_handler)
