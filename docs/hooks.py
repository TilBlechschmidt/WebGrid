import shutil
import os

def on_post_build(*args, **kwargs):
    # Copy latest docker compose file
    site_dir = kwargs['config']['site_dir']
    shutil.copy("distribution/docker/docker-compose.yml", os.path.join(site_dir, "docker-compose.yml"))

    # Copy core documentation to /components/core
    shutil.copytree(".artifacts/core-documentation/doc", os.path.join(site_dir, "rust-doc"))
