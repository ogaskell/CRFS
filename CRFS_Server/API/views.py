import json
from pathlib import Path
from uuid import UUID

from django.conf import settings
from django.http import HttpRequest, HttpResponse, JsonResponse
from django.utils.decorators import method_decorator
from django.views import View
from django.views.decorators.csrf import csrf_exempt

from .handlers import (CheckFSHandler, CheckUserHandler, FetchStateHandler,
                       JSONMessageHandler, PingHandler, PushStateHandler,
                       RegisterFSHandler, RegisterUserHandler)


class GenericJSONView(View):
    """Generic view class for JSON API views."""

    def json_handler(self, request_data: dict, http_method: str) -> tuple[int, dict]:
        """Handler for JSON requests."""
        raise NotImplementedError

    def get(self, request: HttpRequest) -> HttpResponse:
        """Handle GET requests."""
        try:
            request_data = json.loads(request.body.decode())
        except json.decoder.JSONDecodeError:
            return JsonResponse({"code": 8, "err_msg": "Unable to decode JSON."}, status=400)

        code, response_data = self.json_handler(request_data, "GET")
        return JsonResponse(response_data, status=code)

    def post(self, request: HttpRequest) -> HttpResponse:
        """Handle PUSH requests."""
        try:
            request_data = json.loads(request.body.decode())
        except json.decoder.JSONDecodeError:
            return JsonResponse({"code": 8, "err_msg": "Unable to decode JSON."}, status=400)

        code, response_data = self.json_handler(request_data, "POST")
        return JsonResponse(response_data, status=code)


@method_decorator(csrf_exempt, name='dispatch')
class JSONMessageView(GenericJSONView):
    """View designed to handle JSON 'messages'.

    That is, JSON data in a predetermined format, where a 'message' field determines the handler used.
    This class can read said field and call one of a number of predefined callbacks as appropriate.
    """

    def json_handler(self, request_data: dict, http_method: str) -> tuple[int, dict]:
        """Handler for JSON requests."""
        HANDLERS: dict[str, JSONMessageHandler] = {
            "ping": PingHandler,
            "register_user": RegisterUserHandler,
            "check_user": CheckUserHandler,
            "register_fs": RegisterFSHandler,
            "check_fs": CheckFSHandler,
            "fetch_state": FetchStateHandler,
            "push_state": PushStateHandler,
        }

        try:
            type = request_data["type"]
        except KeyError:
            return (400, {"code": 8, "err_msg": "Missing \"type\" field."})

        if type not in HANDLERS.keys():
            return (400, {"code": 8, "err_msg": f"Unrecognised type <{type}>."})

        code, response_data = HANDLERS[type].handle_message(request_data, http_method)
        return (code, response_data)


@csrf_exempt
def operation(request: HttpRequest, fs: UUID, hash: str) -> HttpResponse:
    """Handle /operation/*."""
    ops_dir: Path = settings.BASE_DIR / "operations"
    fs_dir = ops_dir / str(fs)

    fs_dir.mkdir(parents=True, exist_ok=True)

    print(fs_dir.exists())

    filename = fs_dir / hash

    if request.method == "GET":
        print("GET")
        try:
            with open(filename) as f:
                content = f.read()

            return HttpResponse(status=200, content=content)
        except FileNotFoundError:
            return HttpResponse(status=404)
    elif request.method == "PUT":
        print("PUT")
        try:
            with open(filename, "w") as f:
                f.write(request.body.decode())

            return HttpResponse(status=200)
        except Exception as e:
            print(e)
            return HttpResponse(status=500, content=e)
    else:
        return HttpResponse(status=405)
