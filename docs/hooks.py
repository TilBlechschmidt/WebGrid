import shutil
import os

def on_post_build(*args, **kwargs):
    site_dir = kwargs['config']['site_dir']

    # Copy latest docker compose file
    copy_docker_compose(site_dir)

    # Copy core documentation to /components/core
    shutil.copytree(".artifacts/core-documentation/doc", os.path.join(site_dir, "rust-doc"))

def copy_docker_compose(site_dir):
    src_file = "distribution/docker/docker-compose.yml"
    dst_file = os.path.join(site_dir, "docker-compose.yml")

    # Read in the file
    with open(src_file, 'r') as file :
        filedata = file.read()

    # Use the git ref in GitHub Actions
    git_ref = os.getenv('GITHUB_REF')
    if git_ref is not None and git_ref.startswith('refs/tags/'):
        version = git_ref[10:]
        filedata = filedata.replace(':latest', ':v0.0.1')
        print("Overwriting docker-compose version with git tag '" + version + "'")

    # Write the file out again
    with open(dst_file, 'w') as file:
        file.write(filedata)
