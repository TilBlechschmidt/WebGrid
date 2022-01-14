import shutil
import os

def on_pre_build(config):
    # Make sure the chart version is set correctly
    # This is necessary because the Helm package `--version` flag only changes the
    # root chart version but not the subcharts. Thus we need to manually overwrite it!
    print("Running pre-build hook!")
    update_chart_version()

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
        filedata = filedata.replace(':-latest', ':-' + version)
        print("Overwriting docker-compose version with git tag '" + version + "'")

    # Write the file out again
    with open(dst_file, 'w') as file:
        file.write(filedata)

def update_chart_version():
    chart = "distribution/kubernetes/demo/charts/webgrid/Chart.yaml"

    # Read in the file
    with open(chart, 'r') as file :
        filedata = file.read()

    # Use the git ref in GitHub Actions
    git_ref = os.getenv('GITHUB_REF')
    if git_ref is not None and git_ref.startswith('refs/tags/'):
        version = git_ref[10:]
        filedata = filedata.replace('v0.0.0-tempversion', version)
        print("Overwriting Helm subchart version with git tag '" + version + "'")

    # Write the file out again
    with open(chart, 'w') as file:
        file.write(filedata)
