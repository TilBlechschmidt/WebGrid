import shutil
import os

def copy_readme(*args, **kwargs):
    site_dir = kwargs['config']['site_dir']
    shutil.copy("distribution/docker/docker-compose.yml", os.path.join(site_dir, "docker-compose.yml"))
