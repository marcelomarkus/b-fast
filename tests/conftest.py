import os

os.environ.setdefault("DJANGO_SETTINGS_MODULE", "tests.django_settings")

try:
    import django
    from django.conf import settings

    if not settings.configured:
        settings.configure(
            DEBUG=True,
            SECRET_KEY="b-fast-test-secret-key",
            ROOT_URLCONF=[],
        )
    django.setup()
except ImportError:
    pass
