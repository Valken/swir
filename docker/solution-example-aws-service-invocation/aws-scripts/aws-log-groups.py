
import yaml
import subprocess
import time
import random
import string
import sys

cmd = sys.argv[1]

log_groups = ['/swir/swir-library','/swir/magazines','/swir/books']

for log_group in log_groups:

    if cmd == 'CREATE':
        print("Creating log group " + log_group);
        subprocess.call('aws logs create-log-group --log-group ' + log_group ,shell=True)
    if cmd == 'DELETE':
        print("Deleting log group " + log_group);
        subprocess.call('aws logs delete-log-group --log-group ' + log_group ,shell=True)
              
#    print(yaml.dump(data_loaded))
