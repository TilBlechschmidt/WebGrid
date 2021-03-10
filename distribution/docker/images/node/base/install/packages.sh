install_packages software-properties-common

apt-add-repository non-free

PACKAGES=$(grep -v '#' packages.txt | xargs | tr '\n' ' ')
install_packages $PACKAGES
