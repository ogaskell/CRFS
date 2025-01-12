"""API app URL definitions."""
from django.urls import path

from . import views

urlpatterns = [
    path('', views.JSONMessageView.as_view()),
]
