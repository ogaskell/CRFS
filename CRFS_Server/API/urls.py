"""API app URL definitions."""
from django.urls import path

from . import views

urlpatterns = [
    path('api/', views.JSONMessageView.as_view()),
    path('operation/<uuid:fs>/<str:hash>', views.operation),
]
