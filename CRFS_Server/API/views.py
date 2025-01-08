import json
from collections.abc import Callable

from django.http import HttpRequest, HttpResponse, JsonResponse
from django.utils.decorators import method_decorator
from django.views import View
from django.views.decorators.csrf import csrf_exempt


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
            return JsonResponse({}, status=400)

        code, response_data = self.json_handler(request_data, "GET")
        return JsonResponse(response_data, status=code)

    def post(self, request: HttpRequest) -> HttpResponse:
        """Handle PUSH requests."""
        try:
            request_data = json.loads(request.body.decode())
        except json.decoder.JSONDecodeError:
            return JsonResponse({}, status=400)

        code, response_data = self.json_handler(request_data, "POST")
        return JsonResponse(response_data, status=code)


@method_decorator(csrf_exempt, name='dispatch')
class JSONMessageHandler(GenericJSONView):
    """View designed to handle JSON 'messages'.

    That is, JSON data in a predetermined format, where a 'message' field determines the handler used.
    This class can read said field and call one of a number of predefined callbacks as appropriate.
    """

    def json_handler(self, request_data: dict, http_method: str) -> tuple[int, dict]:
        """Handler for JSON requests."""
        HANDLERS: dict[str, Callable[[dict, str], tuple[int, dict]]] = {
            "ping": self.ping_handler,
        }

        try:
            type = request_data["type"]
        except KeyError:
            return (400, {"code": 8, "err_msg": "Missing \"type\" field."})

        if type not in HANDLERS.keys():
            return (400, {"code": 8, "err_msg": f"Unrecognised type <{type}>."})

        code, response_data = HANDLERS[type](request_data, http_method)
        return (code, response_data)

    @staticmethod
    def ping_handler(request_data: dict, http_method: str) -> tuple[int, dict]:
        """Handle `ping` type messages."""
        try:
            id = request_data["transaction_id"]
        except KeyError:
            return (400, {})

        return (200, {
            "version": "1.0",
            "transaction_id": id,
            "reply": True,
            "type": "ping",
            "payload": {},
            "notifications": [],
        })
