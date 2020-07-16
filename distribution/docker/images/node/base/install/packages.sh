PACKAGES=$(grep -v '#' packages.txt | xargs | tr '\n' ' ')
install_packages $PACKAGES
