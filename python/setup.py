from setuptools import setup, find_packages

setup(
    name='y_sweet_sdk',
    version='0.1.0',
    packages=find_packages(),
    install_requires=[
        'requests~=2.32.2',
    ],
    author='Paul Butler',
    author_email='paul@jamsocket.com',
    description='Python bindings for the Y-Sweet server',
    long_description=open('README.md').read(),
    long_description_content_type='text/markdown',
    url='https://github.com/jamsocket/y-sweet',
    classifiers=[
        'Programming Language :: Python :: 3',
        'License :: OSI Approved :: MIT License',
        'Operating System :: OS Independent',
    ],
    python_requires='>=3.6',
)
