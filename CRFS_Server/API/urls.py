"""API app URL definitions."""
from django.urls import path

from . import views

urlpatterns = [
    path('', views.JSONMessageHandler.as_view()),
]
